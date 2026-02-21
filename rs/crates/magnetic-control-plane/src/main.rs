mod auth;
mod caddy;
mod civo;
mod db;
mod error;
mod server;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let port: u16 = env_or("PORT", "4000").parse().expect("invalid PORT");
    let turso_url = std::env::var("TURSO_URL").ok();
    let turso_token = env_or("TURSO_TOKEN", "");
    let civo_key = env_or("CIVO_API_KEY", "");
    let caddy_admin = env_or("CADDY_ADMIN_URL", "http://localhost:2019");
    let domain = env_or("MAGNETIC_DOMAIN", "fujs.dev");

    // Connect to Turso (remote) or local SQLite for dev
    let db = if let Some(ref url) = turso_url {
        eprintln!("[db] connecting to Turso: {}", url);
        db::Db::connect_remote(url, &turso_token)
            .await
            .expect("failed to connect to Turso")
    } else {
        let path = env_or("DB_PATH", "magnetic-control-plane.db");
        eprintln!("[db] using local SQLite: {}", path);
        db::Db::connect_local(&path)
            .await
            .expect("failed to open local db")
    };

    db.init_schema().await.expect("failed to init schema");
    eprintln!("[db] schema ready");

    let http = reqwest::Client::builder()
        .user_agent(format!("magnetic-control-plane/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .pool_max_idle_per_host(0)
        .build()
        .expect("failed to build HTTP client");

    let civo = civo::CivoClient::new(http.clone(), civo_key.clone());
    let caddy = caddy::CaddyManager::new(http.clone(), caddy_admin.clone(), domain.clone(), port);

    let state = Arc::new(server::AppState {
        db: Arc::new(db),
        http,
        civo,
        caddy,
        domain: domain.clone(),
    });

    let app = server::router(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    eprintln!("[control-plane] http://localhost:{}", port);
    eprintln!("[control-plane] Magnetic Cloud Control Plane v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("[control-plane] domain: {}", domain);
    if !civo_key.is_empty() {
        eprintln!("[control-plane] Civo auto-provisioning: enabled");
    } else {
        eprintln!("[control-plane] Civo auto-provisioning: disabled (set CIVO_API_KEY)");
    }
    eprintln!("[control-plane] Caddy admin: {}", caddy_admin);

    axum::serve(listener, app)
        .await
        .expect("server error");
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
