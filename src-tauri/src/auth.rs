//! Optional localhost API token (`NETRAIL_API_TOKEN`).
//! When unset, behavior is unchanged (v1 single-user model).
//! When set, `/api/*` except `/api/health` requires
//! `Authorization: Bearer <token>` or `X-NetRail-Token: <token>`.

use crate::error::{NetRailError, NetRailResult};
use base64::Engine;
use sha2::Digest;
use std::env;

pub fn api_token_from_env() -> Option<String> {
    env::var("NETRAIL_API_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn token_required() -> bool {
    api_token_from_env().is_some()
}

/// Inject UI token into HTML when a token is configured (localhost Docker UX).
pub fn inject_ui_token() -> bool {
    if api_token_from_env().is_none() {
        return false;
    }
    env::var("NETRAIL_INJECT_UI_TOKEN")
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(true)
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let digest_a = sha2::Sha256::digest(a.as_bytes());
    let digest_b = sha2::Sha256::digest(b.as_bytes());
    digest_a
        .iter()
        .zip(digest_b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

pub fn check_request_token(auth_header: Option<&str>, x_token: Option<&str>) -> NetRailResult<()> {
    let Some(expected) = api_token_from_env() else {
        return Ok(());
    };
    if let Some(auth) = auth_header {
        let auth = auth.trim();
        if let Some(bearer) = auth
            .strip_prefix("Bearer ")
            .or_else(|| auth.strip_prefix("bearer "))
        {
            if constant_time_eq(bearer.trim(), &expected) {
                return Ok(());
            }
        }
    }
    if let Some(tok) = x_token {
        if constant_time_eq(tok.trim(), &expected) {
            return Ok(());
        }
    }
    Err(NetRailError::InvalidConfig {
        code: "AUTH_REQUIRED",
        message: "Valid NETRAIL_API_TOKEN required (Authorization: Bearer or X-NetRail-Token)."
            .into(),
    })
}

/// Stable per-client bucket key for rate limiting (A9). When token auth is
/// on, the key is the SHA-256 of the presented token — never the token
/// itself — so each client gets its own per-minute budget. Without auth,
/// everything shares one "anonymous" budget per process.
pub fn client_identity(auth_header: Option<&str>, x_token: Option<&str>) -> String {
    if api_token_from_env().is_none() {
        return "anonymous".into();
    }
    let token = auth_header
        .and_then(|auth| {
            let auth = auth.trim();
            auth.strip_prefix("Bearer ")
                .or_else(|| auth.strip_prefix("bearer "))
        })
        .map(str::trim)
        .or_else(|| x_token.map(str::trim));
    match token {
        Some(t) if !t.is_empty() => {
            let digest = sha2::Sha256::digest(t.as_bytes());
            let encoded = base64::engine::general_purpose::STANDARD.encode(digest);
            format!("token:{encoded}")
        }
        _ => "anonymous".into(),
    }
}

pub fn path_requires_token(path: &str) -> bool {
    if !token_required() {
        return false;
    }
    if path == "/api/health" {
        return false;
    }
    path.starts_with("/api/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn path_health_exempt() {
        // Without env, never required.
        assert!(!path_requires_token("/api/health"));
        assert!(!path_requires_token("/api/search"));
    }

    #[test]
    #[serial_test::serial]
    fn accepts_bearer_when_token_set() {
        std::env::set_var("NETRAIL_API_TOKEN", "secret-test-token");
        assert!(check_request_token(Some("Bearer secret-test-token"), None).is_ok());
        assert!(check_request_token(None, Some("secret-test-token")).is_ok());
        assert!(check_request_token(Some("Bearer wrong"), None).is_err());
        std::env::remove_var("NETRAIL_API_TOKEN");
    }
}
