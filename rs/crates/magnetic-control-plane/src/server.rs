use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::auth::{generate_api_key, generate_id, hash_key, tier_limits, AuthUser};
use crate::caddy::CaddyManager;
use crate::civo::CivoClient;
use crate::db::Db;
use crate::error::AppError;

// ── Shared state ────────────────────────────────────────────────────

pub struct AppState {
    pub db: Arc<Db>,
    pub http: reqwest::Client,
    pub civo: CivoClient,
    pub caddy: CaddyManager,
    pub domain: String,
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Public
        .route("/health", get(health))
        .route("/", get(homepage))
        // Auth
        .route("/api/auth/register", post(register))
        .route("/api/auth/keys", post(create_key))
        .route("/api/auth/me", get(me))
        // Apps
        .route("/api/deploy", post(deploy))
        .route("/api/apps", get(list_apps))
        .route("/api/apps/:id", get(get_app))
        .route("/api/apps/:id", delete(delete_app))
        // Nodes (admin / internal)
        .route("/api/nodes", get(list_nodes))
        .route("/api/nodes", post(register_node))
        .route("/api/nodes/provision", post(provision_node))
        .route("/api/nodes/:id", delete(delete_node))
        // Caddy integration
        .route("/api/resolve/:subdomain", get(resolve_subdomain))
        .route("/api/tls/check", get(tls_check))
        // Caddy sync (manual trigger)
        .route("/api/caddy/sync", post(caddy_sync))
        .with_state(state)
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
}

#[derive(Serialize)]
struct RegisterResponse {
    user_id: String,
    email: String,
    api_key: String,
}

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: Option<String>,
}

#[derive(Serialize)]
struct CreateKeyResponse {
    api_key: String,
    name: String,
}

#[derive(Deserialize)]
pub struct DeployRequest {
    pub name: Option<String>,
    pub bundle: String,
    pub assets: Option<HashMap<String, String>>,
    pub config: Option<String>,
}

#[derive(Serialize)]
struct DeployResponse {
    id: String,
    name: Option<String>,
    url: String,
    node_id: String,
}

#[derive(Serialize)]
struct AppResponse {
    id: String,
    name: Option<String>,
    url: String,
    node_id: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub struct RegisterNodeRequest {
    pub ip: String,
    pub port: Option<i64>,
    pub region: Option<String>,
    pub civo_instance_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ProvisionNodeRequest {
    pub region: Option<String>,
}

#[derive(Serialize)]
struct NodeResponse {
    id: String,
    ip: String,
    port: i64,
    region: String,
    app_count: i64,
    max_apps: i64,
    status: String,
    civo_instance_id: Option<String>,
    created_at: String,
}

#[derive(Serialize)]
struct ResolveResponse {
    app_id: String,
    upstream: String,
}

#[derive(Deserialize)]
pub struct TlsCheckQuery {
    pub domain: Option<String>,
}

// ── Handlers: Public ────────────────────────────────────────────────

async fn health() -> &'static str {
    "ok"
}

async fn homepage(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let nodes = state.db.list_nodes().await.unwrap_or_default();
    let total_apps: i64 = nodes.iter().map(|n| n.app_count).sum();
    let body = serde_json::json!({
        "service": "magnetic-control-plane",
        "version": env!("CARGO_PKG_VERSION"),
        "domain": state.domain,
        "nodes": nodes.len(),
        "apps": total_apps,
    });
    Json(body)
}

// ── Handlers: Auth ──────────────────────────────────────────────────

async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("invalid email".into()));
    }

    // Check if user exists
    if let Some(existing) = state.db.get_user_by_email(&email).await? {
        // Generate a new key for existing user
        let api_key = generate_api_key();
        let key_hash = hash_key(&api_key);
        state
            .db
            .store_api_key(&key_hash, &existing.id, "default")
            .await?;
        return Ok((
            StatusCode::OK,
            Json(RegisterResponse {
                user_id: existing.id,
                email: existing.email,
                api_key,
            }),
        ));
    }

    let user_id = generate_id(12);
    state.db.create_user(&user_id, &email).await?;

    let api_key = generate_api_key();
    let key_hash = hash_key(&api_key);
    state
        .db
        .store_api_key(&key_hash, &user_id, "default")
        .await?;

    eprintln!("[auth] registered user {} ({})", user_id, email);

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id,
            email,
            api_key,
        }),
    ))
}

async fn create_key(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, AppError> {
    let name = req.name.unwrap_or_else(|| "unnamed".into());
    let api_key = generate_api_key();
    let key_hash = hash_key(&api_key);
    state
        .db
        .store_api_key(&key_hash, &user.id, &name)
        .await?;
    Ok(Json(CreateKeyResponse { api_key, name }))
}

async fn me(AuthUser(user): AuthUser) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "tier": user.tier,
    }))
}

// ── Handlers: Apps ──────────────────────────────────────────────────

async fn deploy(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(req): Json<DeployRequest>,
) -> Result<(StatusCode, Json<DeployResponse>), AppError> {
    if req.bundle.is_empty() {
        return Err(AppError::BadRequest("bundle is required".into()));
    }

    // Bundle size limit: 5MB
    if req.bundle.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest("bundle exceeds 5MB limit".into()));
    }

    // Check tier limits
    let limits = tier_limits(&user.tier);
    let app_count = state.db.count_apps_for_user(&user.id).await?;

    // Check if this is a redeploy of an existing app (by name)
    let existing = if let Some(ref name) = req.name {
        state.db.get_app_by_name(name).await?
    } else {
        None
    };

    let is_redeploy = existing
        .as_ref()
        .map(|a| a.user_id == user.id)
        .unwrap_or(false);

    if !is_redeploy && app_count >= limits.max_apps {
        return Err(AppError::Forbidden(format!(
            "tier '{}' allows max {} apps",
            user.tier, limits.max_apps
        )));
    }

    // If name is taken by another user, reject
    if let Some(ref ex) = existing {
        if ex.user_id != user.id {
            return Err(AppError::Forbidden("app name taken".into()));
        }
    }

    // Select or provision a node
    let node = match state.db.select_node().await? {
        Some(n) => n,
        None => {
            // Try auto-provisioning via Civo
            if state.civo.is_configured() {
                eprintln!("[deploy] no available nodes, provisioning via Civo...");
                let hostname = format!("magnetic-node-{}", generate_id(6));
                let inst = state.civo.provision(&hostname, "LON1").await?;
                eprintln!(
                    "[deploy] provisioned {} ({}), waiting for ready...",
                    inst.hostname, inst.id
                );
                let ready = state.civo.wait_until_ready(&inst.id, 300).await?;
                let node_id = generate_id(8);
                let node = state
                    .db
                    .create_node(&node_id, &ready.public_ip, 3003, "LON1", Some(&inst.id))
                    .await?;
                eprintln!("[deploy] node {} ready at {}:3003", node_id, ready.public_ip);
                node
            } else {
                return Err(AppError::Internal(
                    "no available nodes and Civo not configured".into(),
                ));
            }
        }
    };

    // Determine app ID
    let (app_id, created) = if let Some(ref ex) = existing {
        if is_redeploy {
            (ex.id.clone(), false)
        } else {
            (generate_id(8), true)
        }
    } else {
        (generate_id(8), true)
    };

    // Deploy to node: POST /api/apps/{app_id}/deploy
    let node_url = format!("http://{}:{}", node.ip, node.port);
    let deploy_url = format!("{}/api/apps/{}/deploy", node_url, app_id);
    let bundle_len = req.bundle.len();
    let has_config = req.config.is_some();
    let asset_count = req.assets.as_ref().map(|a| a.len()).unwrap_or(0);

    eprintln!(
        "[deploy] forwarding to {} (bundle={}B, assets={}, config={})",
        deploy_url, bundle_len, asset_count, has_config
    );

    let deploy_payload = serde_json::json!({
        "bundle": req.bundle,
        "assets": req.assets.as_ref().unwrap_or(&HashMap::new()),
        "config": req.config.as_deref().unwrap_or(""),
    });

    let t0 = std::time::Instant::now();
    let resp = state
        .http
        .post(&deploy_url)
        .json(&deploy_payload)
        .send()
        .await
        .map_err(|e| {
            let elapsed = t0.elapsed();
            eprintln!(
                "[deploy] ✗ node request failed after {:.1}s: {} (url={})",
                elapsed.as_secs_f64(), e, deploy_url
            );
            AppError::Upstream(format!("node deploy failed: {}", e))
        })?;

    let elapsed = t0.elapsed();
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        eprintln!(
            "[deploy] ✗ node returned {} after {:.1}s: {} (url={})",
            status, elapsed.as_secs_f64(), body, deploy_url
        );
        return Err(AppError::Upstream(format!(
            "node returned {} : {}",
            status, body
        )));
    }

    eprintln!("[deploy] ✓ node responded 200 in {:.1}s", elapsed.as_secs_f64());

    // Update DB
    if created {
        state
            .db
            .create_app(&app_id, req.name.as_deref(), &user.id, &node.id)
            .await?;
        state.db.increment_node_app_count(&node.id).await?;
    } else {
        // Redeploy — update timestamp
        state.db.update_app_node(&app_id, &node.id).await?;
    }

    let url = format!("https://{}.{}", app_id, state.domain);

    // NOTE: Caddy config push disabled. The Caddyfile already handles all
    // subdomain routing via *.fujs.dev wildcard with DNS-challenge TLS,
    // gzip/zstd compression, and SSE flush_interval. The dynamic config
    // push was replacing the entire Caddyfile, wiping wildcard TLS and
    // triggering per-domain cert issuance that blocked subsequent deploys.
    // if let Some(app) = state.db.get_app(&app_id).await? {
    //     let _ = state.caddy.add_app(&app, &node, &state.db).await;
    // }

    eprintln!(
        "[deploy] {} app '{}' (id={}) on node {} → {}",
        if created { "created" } else { "redeployed" },
        req.name.as_deref().unwrap_or("-"),
        app_id,
        node.id,
        url
    );

    Ok((
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(DeployResponse {
            id: app_id,
            name: req.name,
            url,
            node_id: node.id,
        }),
    ))
}

async fn list_apps(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<AppResponse>>, AppError> {
    let apps = state.db.list_apps_for_user(&user.id).await?;
    let responses: Vec<AppResponse> = apps
        .into_iter()
        .map(|a| {
            let url = format!("https://{}.{}", a.id, state.domain);
            AppResponse {
                id: a.id,
                name: a.name,
                url,
                node_id: a.node_id,
                created_at: a.created_at,
                updated_at: a.updated_at,
            }
        })
        .collect();
    Ok(Json(responses))
}

async fn get_app(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<AppResponse>, AppError> {
    let app = state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("app not found".into()))?;
    if app.user_id != user.id {
        return Err(AppError::Forbidden("not your app".into()));
    }
    let url = format!("https://{}.{}", app.id, state.domain);
    Ok(Json(AppResponse {
        id: app.id,
        name: app.name,
        url,
        node_id: app.node_id,
        created_at: app.created_at,
        updated_at: app.updated_at,
    }))
}

async fn delete_app(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let app = state
        .db
        .get_app(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("app not found".into()))?;
    if app.user_id != user.id {
        return Err(AppError::Forbidden("not your app".into()));
    }

    state.db.delete_app(&id).await?;
    state.db.decrement_node_app_count(&app.node_id).await?;

    // NOTE: Caddy config push disabled (see deploy handler comment).
    // let _ = state.caddy.remove_app(&state.db).await;

    eprintln!("[apps] deleted app {} (node={})", id, app.node_id);
    Ok(StatusCode::NO_CONTENT)
}

// ── Handlers: Nodes ─────────────────────────────────────────────────

async fn list_nodes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<NodeResponse>>, AppError> {
    let nodes = state.db.list_nodes().await?;
    Ok(Json(
        nodes
            .into_iter()
            .map(|n| NodeResponse {
                id: n.id,
                ip: n.ip,
                port: n.port,
                region: n.region,
                app_count: n.app_count,
                max_apps: n.max_apps,
                status: n.status,
                civo_instance_id: n.civo_instance_id,
                created_at: n.created_at,
            })
            .collect(),
    ))
}

async fn register_node(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterNodeRequest>,
) -> Result<(StatusCode, Json<NodeResponse>), AppError> {
    let node_id = generate_id(8);
    let port = req.port.unwrap_or(3003);
    let region = req.region.as_deref().unwrap_or("LON1");
    let node = state
        .db
        .create_node(&node_id, &req.ip, port, region, req.civo_instance_id.as_deref())
        .await?;
    eprintln!(
        "[nodes] registered node {} ({}:{})",
        node_id, req.ip, port
    );
    Ok((
        StatusCode::CREATED,
        Json(NodeResponse {
            id: node.id,
            ip: node.ip,
            port: node.port,
            region: node.region,
            app_count: node.app_count,
            max_apps: node.max_apps,
            status: node.status,
            civo_instance_id: node.civo_instance_id,
            created_at: node.created_at,
        }),
    ))
}

async fn provision_node(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ProvisionNodeRequest>,
) -> Result<(StatusCode, Json<NodeResponse>), AppError> {
    if !state.civo.is_configured() {
        return Err(AppError::BadRequest("CIVO_API_KEY not configured".into()));
    }

    let region = req.region.as_deref().unwrap_or("LON1");
    let hostname = format!("magnetic-node-{}", generate_id(6));

    eprintln!("[nodes] provisioning {} in {}...", hostname, region);
    let inst = state.civo.provision(&hostname, region).await?;
    eprintln!("[nodes] waiting for {} to be ready...", inst.id);
    let ready = state.civo.wait_until_ready(&inst.id, 300).await?;

    let node_id = generate_id(8);
    let node = state
        .db
        .create_node(&node_id, &ready.public_ip, 3003, region, Some(&inst.id))
        .await?;

    eprintln!(
        "[nodes] provisioned {} at {}:3003 (civo={})",
        node_id, ready.public_ip, inst.id
    );

    Ok((
        StatusCode::CREATED,
        Json(NodeResponse {
            id: node.id,
            ip: node.ip,
            port: node.port,
            region: node.region,
            app_count: node.app_count,
            max_apps: node.max_apps,
            status: node.status,
            civo_instance_id: node.civo_instance_id,
            created_at: node.created_at,
        }),
    ))
}

async fn delete_node(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let nodes = state.db.list_nodes().await?;
    let node = nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or_else(|| AppError::NotFound("node not found".into()))?;

    // Destroy Civo instance if applicable
    if let Some(ref civo_id) = node.civo_instance_id {
        if state.civo.is_configured() {
            eprintln!("[nodes] destroying Civo instance {}...", civo_id);
            state.civo.destroy_instance(civo_id).await?;
        }
    }

    state.db.delete_node(&id).await?;
    eprintln!("[nodes] deleted node {}", id);
    Ok(StatusCode::NO_CONTENT)
}

// ── Handlers: Caddy integration ─────────────────────────────────────

/// Caddy calls this to resolve a subdomain to an upstream address.
/// Used as a fallback / debug endpoint.
async fn resolve_subdomain(
    State(state): State<Arc<AppState>>,
    Path(subdomain): Path<String>,
) -> Result<Json<ResolveResponse>, AppError> {
    let (app, node) = state
        .db
        .resolve_subdomain(&subdomain)
        .await?
        .ok_or_else(|| AppError::NotFound("unknown subdomain".into()))?;

    Ok(Json(ResolveResponse {
        app_id: app.id,
        upstream: format!("{}:{}", node.ip, node.port),
    }))
}

/// on_demand_tls ask endpoint — Caddy queries this to decide if it should
/// provision a TLS certificate for a given domain.
async fn tls_check(
    State(state): State<Arc<AppState>>,
    Query(q): Query<TlsCheckQuery>,
) -> impl IntoResponse {
    let domain = q.domain.unwrap_or_default();
    let subdomain = domain
        .strip_suffix(&format!(".{}", state.domain))
        .unwrap_or("");

    if subdomain.is_empty() {
        return StatusCode::FORBIDDEN;
    }

    if crate::caddy::check_tls_allowed(&state.db, subdomain).await {
        StatusCode::OK
    } else {
        StatusCode::FORBIDDEN
    }
}

async fn caddy_sync(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    state.caddy.sync_routes(&state.db).await?;
    Ok(StatusCode::OK)
}
