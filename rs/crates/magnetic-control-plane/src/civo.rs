//! Civo API client for node auto-provisioning.
//!
//! We use plain Civo compute instances (not K3s clusters) because:
//! - Each node runs a single `magnetic-v8-server --platform` binary
//! - No container orchestration needed — our control plane handles scheduling
//! - Lower overhead (~10MB per V8 isolate vs ~200MB K3s base)
//! - Simpler networking (direct IP, no service mesh)
//!
//! K3s would make sense if we needed rolling deployments of the server binary
//! itself or multi-container workloads, but that's not our model.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const CIVO_API: &str = "https://api.civo.com/v2";

/// Bootstrap script run on new instances. Installs and starts magnetic-v8-server.
const INIT_SCRIPT: &str = r#"#!/bin/bash
set -euo pipefail

# Download latest magnetic-v8-server binary
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  *)       echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

RELEASE_URL="https://github.com/nicholasgriffintn/magnetic/releases/latest/download/magnetic-v8-server-${TARGET}"
curl -fsSL "$RELEASE_URL" -o /usr/local/bin/magnetic-v8-server
chmod +x /usr/local/bin/magnetic-v8-server

# Create data directory
mkdir -p /var/lib/magnetic/apps

# Create systemd service
cat > /etc/systemd/system/magnetic-platform.service << 'EOF'
[Unit]
Description=Magnetic Platform Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/magnetic-v8-server --platform --port 3003 --data-dir /var/lib/magnetic/apps
Restart=always
RestartSec=2
LimitNOFILE=65535
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable magnetic-platform
systemctl start magnetic-platform
"#;

pub struct CivoClient {
    http: reqwest::Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CivoInstance {
    pub id: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub public_ip: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub region: String,
}

#[derive(Debug, Deserialize)]
struct CivoInstanceResponse {
    id: String,
    #[serde(default)]
    hostname: String,
    #[serde(default)]
    public_ip: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    region: String,
}

#[derive(Debug, Deserialize)]
struct CivoListResponse {
    #[serde(default)]
    items: Vec<CivoInstanceResponse>,
}

#[derive(Debug, Deserialize)]
struct CivoDiskImageResponse {
    id: String,
    #[serde(default)]
    name: String,
}

impl CivoClient {
    pub fn new(http: reqwest::Client, api_key: String) -> Self {
        Self { http, api_key }
    }

    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Provision a new Civo instance for running magnetic-v8-server --platform.
    /// Uses g3.medium (2 vCPU, 4GB RAM) — holds ~300 warm V8 isolates.
    pub async fn provision(
        &self,
        hostname: &str,
        region: &str,
    ) -> Result<CivoInstance, AppError> {
        if !self.is_configured() {
            return Err(AppError::Internal("CIVO_API_KEY not configured".into()));
        }

        // Civo uses lowercase region codes
        let region_lower = region.to_lowercase();

        // Find Ubuntu 22.04 template
        let template_id = self.find_ubuntu_template(&region_lower).await?;

        eprintln!("[civo] creating instance: hostname={}, region={}, template={}", hostname, region_lower, template_id);

        // Base64-encode the init script — Civo stores scripts as base64 and
        // their nginx WAF blocks large plaintext form bodies.
        let script_b64 = STANDARD.encode(INIT_SCRIPT);

        // Civo instance creation uses form-encoded body
        let resp = self
            .http
            .post(format!("{}/instances", CIVO_API))
            .bearer_auth(&self.api_key)
            .form(&[
                ("hostname", hostname),
                ("size", "g3.medium"),
                ("template_id", &template_id),
                ("region", &region_lower),
                ("script", script_b64.as_str()),
                ("count", "1"),
                ("public_ip", "create"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "civo create instance: {} {}",
                status, text
            )));
        }

        let text = resp.text().await.unwrap_or_default();
        eprintln!("[civo] create response: {}", &text[..text.len().min(500)]);
        let inst: CivoInstanceResponse = serde_json::from_str(&text)
            .map_err(|e| AppError::Upstream(format!("civo parse: {} — body: {}", e, &text[..text.len().min(200)])))?;
        Ok(CivoInstance {
            id: inst.id,
            hostname: inst.hostname,
            public_ip: inst.public_ip,
            status: inst.status,
            region: inst.region,
        })
    }

    /// Poll until instance has a public IP and is ACTIVE.
    pub async fn wait_until_ready(
        &self,
        instance_id: &str,
        timeout_secs: u64,
    ) -> Result<CivoInstance, AppError> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(AppError::Upstream(format!(
                    "civo instance {} not ready after {}s",
                    instance_id, timeout_secs
                )));
            }

            let inst = self.get_instance(instance_id).await?;
            if inst.status == "ACTIVE" && !inst.public_ip.is_empty() {
                return Ok(inst);
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    pub async fn get_instance(&self, id: &str) -> Result<CivoInstance, AppError> {
        let resp = self
            .http
            .get(format!("{}/instances/{}", CIVO_API, id))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "civo get instance: {} {}",
                status, text
            )));
        }

        let inst: CivoInstanceResponse = resp.json().await
            .map_err(|e| AppError::Upstream(format!("civo get instance parse: {}", e)))?;
        Ok(CivoInstance {
            id: inst.id,
            hostname: inst.hostname,
            public_ip: inst.public_ip,
            status: inst.status,
            region: inst.region,
        })
    }

    pub async fn destroy_instance(&self, id: &str) -> Result<(), AppError> {
        let resp = self
            .http
            .delete(format!("{}/instances/{}", CIVO_API, id))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "civo destroy instance: {} {}",
                status, text
            )));
        }
        Ok(())
    }

    pub async fn list_instances(&self) -> Result<Vec<CivoInstance>, AppError> {
        let resp = self
            .http
            .get(format!("{}/instances", CIVO_API))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "civo list instances: {} {}",
                status, text
            )));
        }

        let list: CivoListResponse = resp.json().await
            .map_err(|e| AppError::Upstream(format!("civo list parse: {}", e)))?;
        Ok(list
            .items
            .into_iter()
            .map(|i| CivoInstance {
                id: i.id,
                hostname: i.hostname,
                public_ip: i.public_ip,
                status: i.status,
                region: i.region,
            })
            .collect())
    }

    async fn find_ubuntu_template(&self, region: &str) -> Result<String, AppError> {
        let resp = self
            .http
            .get(format!("{}/disk_images", CIVO_API))
            .bearer_auth(&self.api_key)
            .query(&[("region", region)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::Upstream("civo: failed to list disk images".into()));
        }

        // Civo disk_images returns a plain array, not { items: [...] }
        let images: Vec<CivoDiskImageResponse> = resp.json().await
            .map_err(|e| AppError::Upstream(format!("civo disk images parse: {}", e)))?;

        // Prefer Ubuntu 22.04, fall back to any Ubuntu
        if let Some(img) = images.iter().find(|i| i.name.contains("ubuntu-jammy")) {
            return Ok(img.id.clone());
        }
        if let Some(img) = images.iter().find(|i| i.name.to_lowercase().contains("ubuntu")) {
            return Ok(img.id.clone());
        }

        Err(AppError::Upstream(
            "civo: no Ubuntu disk image found".into(),
        ))
    }
}
