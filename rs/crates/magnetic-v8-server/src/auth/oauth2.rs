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

// ── Magic-link / Token-based auth ───────────────────────────────────

/// Verify a token received directly in the callback URL (magic-link style).
/// Calls the provider's verify/authenticate endpoint with the token.
/// Returns (session_token, None, expires_in).
pub fn verify_magic_link_token(
    verify_url: &str,
    token: &str,
    token_field: &str,
    default_expires: u64,
) -> Result<(String, Option<String>, u64), String> {
    eprintln!("[auth] verifying magic-link token at {}", verify_url);

    let body = serde_json::json!({ "token": token });

    let resp = ureq::post(verify_url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("magic-link verify failed: {}", e))?;

    let text = resp.into_string()
        .map_err(|e| format!("magic-link verify read: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("magic-link verify parse: {}", e))?;

    // Try the configured token_field, then common alternatives
    let session_token = json.get(token_field)
        .or_else(|| json.get("session_token"))
        .or_else(|| json.get("access_token"))
        .or_else(|| json.get("token"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| format!("No '{}' in magic-link verify response", token_field))?
        .to_string();

    let expires_in = json.get("expires_in")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(default_expires);

    eprintln!("[auth] magic-link verified (expires_in={}s)", expires_in);
    Ok((session_token, None, expires_in))
}

// ── OTP / API-based auth ────────────────────────────────────────────

/// Send an OTP or magic-link email via the provider's login API.
/// Returns the response body (may contain a method_id or status).
pub fn send_otp(
    login_url: &str,
    email: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<serde_json::Value, String> {
    eprintln!("[auth] sending OTP/magic-link to {} via {}", email, login_url);

    let body = serde_json::json!({ "email": email });

    let resp = ureq::post(login_url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Basic {}", base64_encode(&format!("{}:{}", client_id, client_secret))))
        .send_string(&body.to_string())
        .map_err(|e| format!("OTP send failed: {}", e))?;

    let text = resp.into_string()
        .map_err(|e| format!("OTP send read: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("OTP send parse: {}", e))?;

    eprintln!("[auth] OTP/magic-link sent successfully");
    Ok(json)
}

/// Verify an OTP code via the provider's verify API.
/// Returns (session_token, None, expires_in).
pub fn verify_otp(
    verify_url: &str,
    code: &str,
    method_id: &str,
    client_id: &str,
    client_secret: &str,
    token_field: &str,
    default_expires: u64,
) -> Result<(String, Option<String>, u64), String> {
    eprintln!("[auth] verifying OTP at {}", verify_url);

    let body = serde_json::json!({
        "code": code,
        "method_id": method_id,
    });

    let resp = ureq::post(verify_url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Basic {}", base64_encode(&format!("{}:{}", client_id, client_secret))))
        .send_string(&body.to_string())
        .map_err(|e| format!("OTP verify failed: {}", e))?;

    let text = resp.into_string()
        .map_err(|e| format!("OTP verify read: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("OTP verify parse: {}", e))?;

    let session_token = json.get(token_field)
        .or_else(|| json.get("session_token"))
        .or_else(|| json.get("access_token"))
        .or_else(|| json.get("token"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| format!("No '{}' in OTP verify response", token_field))?
        .to_string();

    let expires_in = json.get("expires_in")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(default_expires);

    eprintln!("[auth] OTP verified (expires_in={}s)", expires_in);
    Ok((session_token, None, expires_in))
}

/// Minimal base64 encoder (for Basic auth header).
fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
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
