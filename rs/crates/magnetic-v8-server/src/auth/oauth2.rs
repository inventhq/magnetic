//! oauth2.rs — OAuth2/OIDC token exchange
//!
//! Handles:
//! - OIDC discovery (.well-known/openid-configuration)
//! - Authorization code → token exchange
//! - Token refresh

/// Discover the authorization endpoint from OIDC .well-known configuration.
pub fn discover_auth_endpoint(issuer: &str) -> Result<String, String> {
    let url = format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/'));
    let resp = ureq::get(&url)
        .call()
        .map_err(|e| format!("OIDC discovery failed: {}", e))?;
    let text = resp.into_string()
        .map_err(|e| format!("OIDC discovery read: {}", e))?;
    let body: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("OIDC discovery parse: {}", e))?;
    body.get("authorization_endpoint")
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(|s: &str| s.to_string())
        .ok_or_else(|| "No authorization_endpoint in OIDC discovery".into())
}

/// Discover the token endpoint from OIDC .well-known configuration.
fn discover_token_endpoint(issuer: &str) -> Result<String, String> {
    let url = format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/'));
    let resp = ureq::get(&url)
        .call()
        .map_err(|e| format!("OIDC discovery failed: {}", e))?;
    let text = resp.into_string()
        .map_err(|e| format!("OIDC discovery read: {}", e))?;
    let body: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("OIDC discovery parse: {}", e))?;
    body.get("token_endpoint")
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(|s: &str| s.to_string())
        .ok_or_else(|| "No token_endpoint in OIDC discovery".into())
}

/// Exchange an authorization code for access + refresh tokens.
/// Returns (access_token, refresh_token, expires_in_secs).
pub fn exchange_code(
    issuer: &str,
    provider: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<(String, Option<String>, u64), String> {
    let token_url = if provider == "oidc" {
        discover_token_endpoint(issuer)?
    } else {
        format!("{}/token", issuer.trim_end_matches('/'))
    };

    let body = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&client_secret={}",
        urlencoding(code),
        urlencoding(redirect_uri),
        urlencoding(client_id),
        urlencoding(client_secret),
    );

    eprintln!("[auth] exchanging code at {}", token_url);

    let resp = ureq::post(&token_url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
        .map_err(|e| format!("token exchange failed: {}", e))?;

    let text = resp.into_string()
        .map_err(|e| format!("token response read: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("token response parse: {}", e))?;

    let access_token = json.get("access_token")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or("No access_token in response")?
        .to_string();

    let refresh_token = json.get("refresh_token")
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(|s: &str| s.to_string());

    let expires_in = json.get("expires_in")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(3600);

    eprintln!("[auth] token exchange successful (expires_in={}s)", expires_in);
    Ok((access_token, refresh_token, expires_in))
}

/// Refresh an access token using a refresh token.
/// Returns (new_access_token, new_refresh_token, expires_in_secs).
pub fn refresh_token(
    issuer: &str,
    provider: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<(String, Option<String>, u64), String> {
    let token_url = if provider == "oidc" {
        discover_token_endpoint(issuer)?
    } else {
        format!("{}/token", issuer.trim_end_matches('/'))
    };

    let body = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}&client_secret={}",
        urlencoding(refresh_token),
        urlencoding(client_id),
        urlencoding(client_secret),
    );

    eprintln!("[auth] refreshing token at {}", token_url);

    let resp = ureq::post(&token_url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
        .map_err(|e| format!("token refresh failed: {}", e))?;

    let text = resp.into_string()
        .map_err(|e| format!("refresh response read: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("refresh response parse: {}", e))?;

    let access_token = json.get("access_token")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or("No access_token in refresh response")?
        .to_string();

    let new_refresh = json.get("refresh_token")
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(|s: &str| s.to_string());

    let expires_in = json.get("expires_in")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(3600);

    eprintln!("[auth] token refresh successful (expires_in={}s)", expires_in);
    Ok((access_token, new_refresh, expires_in))
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}
