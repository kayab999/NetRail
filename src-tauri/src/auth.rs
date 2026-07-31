//! Optional localhost API token (`NETRAIL_API_TOKEN`).
//! When unset, behavior is unchanged (v1 single-user model).
//! When set, `/api/*` except `/api/health` requires
//! `Authorization: Bearer <token>` or `X-NetRail-Token: <token>`.

use crate::error::{NetRailError, NetRailResult};
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
            if bearer.trim() == expected {
                return Ok(());
            }
        }
    }
    if let Some(tok) = x_token {
        if tok.trim() == expected {
            return Ok(());
        }
    }
    Err(NetRailError::InvalidConfig {
        code: "AUTH_REQUIRED",
        message: "Valid NETRAIL_API_TOKEN required (Authorization: Bearer or X-NetRail-Token)."
            .into(),
    })
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
