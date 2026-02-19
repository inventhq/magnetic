//! Caddy admin API integration.
//!
//! On every deploy/undeploy, we push updated route config to Caddy via its
//! admin API (localhost:2019). This gives zero per-request overhead — Caddy
//! has the full routing table in memory and routes directly to nodes.

use serde_json::{json, Value};

use crate::db::{App, Db, Node};
use crate::error::AppError;

pub struct CaddyManager {
    http: reqwest::Client,
    admin_url: String,
    domain: String,
    control_plane_port: u16,
}

impl CaddyManager {
    pub fn new(
        http: reqwest::Client,
        admin_url: String,
        domain: String,
        control_plane_port: u16,
    ) -> Self {
        Self {
            http,
            admin_url,
            domain,
            control_plane_port,
        }
    }

    /// Rebuild and push the entire Caddy route config from current DB state.
    pub async fn sync_routes(&self, db: &Db) -> Result<(), AppError> {
        let nodes = db.list_nodes().await?;
        let mut app_routes: Vec<Value> = Vec::new();

        for node in &nodes {
            let apps = db.list_apps_on_node(&node.id).await?;
            for app in &apps {
                app_routes.extend(self.app_route_entries(app, node));
            }
        }

        let config = self.build_config(app_routes);
        self.push_config(&config).await
    }

    /// Add routes for a single newly-deployed app and push to Caddy.
    pub async fn add_app(&self, _app: &App, _node: &Node, db: &Db) -> Result<(), AppError> {
        // Full rebuild is simpler and atomic — avoids partial state.
        // Caddy config loads are fast (<1ms) so this is fine even at scale.
        self.sync_routes(db).await
    }

    /// Remove routes for a deleted app.
    pub async fn remove_app(&self, db: &Db) -> Result<(), AppError> {
        self.sync_routes(db).await
    }

    fn app_route_entries(&self, app: &App, node: &Node) -> Vec<Value> {
        let upstream = format!("{}:{}", node.ip, node.port);
        let mut entries = Vec::with_capacity(2);

        // Route by app ID subdomain: {app_id}.magnetic.app → node
        entries.push(self.make_route(&app.id, &upstream));

        // Route by vanity name if set: {name}.magnetic.app → node
        if let Some(ref name) = app.name {
            entries.push(self.make_route(name, &upstream));
        }

        entries
    }

    fn make_route(&self, subdomain: &str, upstream: &str) -> Value {
        let host = format!("{}.{}", subdomain, self.domain);
        json!({
            "match": [{ "host": [host] }],
            "handle": [
                {
                    "handler": "rewrite",
                    "uri": format!("/apps/{}{{http.request.uri}}", subdomain)
                },
                {
                    "handler": "reverse_proxy",
                    "upstreams": [{ "dial": upstream }],
                    "transport": {
                        "protocol": "http"
                    }
                }
            ],
            "terminal": true
        })
    }

    fn build_config(&self, app_routes: Vec<Value>) -> Value {
        let mut routes = Vec::new();

        // 1. Control plane API route
        routes.push(json!({
            "match": [{ "host": [format!("api.{}", self.domain)] }],
            "handle": [{
                "handler": "reverse_proxy",
                "upstreams": [{ "dial": format!("localhost:{}", self.control_plane_port) }]
            }],
            "terminal": true
        }));

        // 2. Platform homepage
        routes.push(json!({
            "match": [{ "host": [&self.domain] }],
            "handle": [{
                "handler": "reverse_proxy",
                "upstreams": [{ "dial": format!("localhost:{}", self.control_plane_port) }]
            }],
            "terminal": true
        }));

        // 3. Per-app routes (dynamically generated)
        routes.extend(app_routes);

        // 4. Fallback 404
        routes.push(json!({
            "handle": [{
                "handler": "static_response",
                "status_code": "404",
                "headers": { "Content-Type": ["application/json"] },
                "body": "{\"error\":\"app not found\"}"
            }]
        }));

        json!({
            "apps": {
                "http": {
                    "servers": {
                        "magnetic": {
                            "listen": [":443", ":80"],
                            "routes": routes
                        }
                    }
                }
            }
        })
    }

    async fn push_config(&self, config: &Value) -> Result<(), AppError> {
        let resp = self
            .http
            .post(format!("{}/load", self.admin_url))
            .header("Content-Type", "application/json")
            .json(config)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                eprintln!("[caddy] config pushed successfully");
                Ok(())
            }
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                eprintln!("[caddy] config push failed: {} {}", status, body);
                // Non-fatal: app is deployed even if Caddy update fails.
                // Operator can manually sync later.
                Ok(())
            }
            Err(e) => {
                eprintln!("[caddy] config push error (caddy down?): {}", e);
                // Non-fatal: same reasoning as above
                Ok(())
            }
        }
    }
}

/// Check if a subdomain should get a TLS certificate (on_demand_tls ask endpoint).
pub async fn check_tls_allowed(db: &Db, subdomain: &str) -> bool {
    db.resolve_subdomain(subdomain).await.ok().flatten().is_some()
}
