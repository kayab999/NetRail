use crate::error::{NetRailError, NetRailResult};
use std::net::IpAddr;
use url::Url;

const DDG_HOSTS: &[&str] = &["duckduckgo.com", "r.duckduckgo.com"];
const MAX_REDIRECT_DEPTH: u8 = 5;

pub fn validate_open_url(raw: &str) -> NetRailResult<String> {
    validate_open_url_inner(raw.trim(), 0)
}

fn validate_open_url_inner(raw: &str, depth: u8) -> NetRailResult<String> {
    if depth > MAX_REDIRECT_DEPTH {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_REDIRECT_DEPTH",
            message: "Too many redirect wrappers.".into(),
        });
    }

    let parsed = Url::parse(raw).map_err(|_| NetRailError::InvalidOpenUrl {
        code: "OPEN_URL_INVALID",
        message: "Invalid URL.".into(),
    })?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_INVALID_SCHEME",
            message: "Only http:// and https:// URLs are supported.".into(),
        });
    }

    if parsed.username() != "" || parsed.password().is_some() {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_CREDENTIALS",
            message: "URLs with embedded credentials are not allowed.".into(),
        });
    }

    if let Some(host) = parsed.host_str() {
        let host_lower = host.to_lowercase();
        if is_ddg_host(&host_lower) {
            if let Some((_, uddg)) = parsed.query_pairs().find(|(k, _)| k == "uddg") {
                return validate_open_url_inner(&uddg, depth + 1);
            }
        }
    }

    block_unsafe_host(parsed.host_str().ok_or_else(|| NetRailError::InvalidOpenUrl {
        code: "OPEN_URL_NO_HOST",
        message: "URL must include a host.".into(),
    })?)?;

    Ok(raw.to_string())
}

fn is_ddg_host(host: &str) -> bool {
    DDG_HOSTS.iter().any(|&h| host == h || host.ends_with(&format!(".{h}")))
}

fn block_unsafe_host(host: &str) -> NetRailResult<()> {
    let host_lower = host.to_lowercase();

    if matches!(
        host_lower.as_str(),
        "localhost" | "127.0.0.1" | "::1" | "0.0.0.0" | "[::1]"
    ) {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_LOCALHOST",
            message: "Localhost URLs cannot be opened from search results.".into(),
        });
    }

    if host_lower.ends_with(".nip.io")
        || host_lower.ends_with(".sslip.io")
        || host_lower.ends_with(".xip.io")
    {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_DNS_REBINDING",
            message: "DNS rebinding hostnames cannot be opened from search results.".into(),
        });
    }

    if let Ok(ip) = host_lower.parse::<IpAddr>() {
        let link_local = match ip {
            IpAddr::V4(v4) => v4.is_link_local(),
            IpAddr::V6(v6) => v6.is_unicast_link_local(),
        };
        if ip.is_loopback() || ip.is_unspecified() || link_local {
            return Err(NetRailError::InvalidOpenUrl {
                code: "OPEN_URL_LINK_LOCAL",
                message: "Local or link-local IP addresses cannot be opened from search results."
                    .into(),
            });
        }
    }

    Ok(())
}

/// Validate a user-configured backend URL (e.g. SearXNG). Localhost is allowed;
/// cloud metadata, rebinding hostnames, and link-local addresses are blocked.
pub fn validate_backend_url(raw: &str) -> NetRailResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_EMPTY",
            message: "Backend URL cannot be empty.".into(),
        });
    }

    let parsed = Url::parse(trimmed).map_err(|_| NetRailError::InvalidBackendUrl {
        code: "BACKEND_URL_INVALID",
        message: "Invalid backend URL.".into(),
    })?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_INVALID_SCHEME",
            message: "Backend URL must use http:// or https://.".into(),
        });
    }

    if parsed.username() != "" || parsed.password().is_some() {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_CREDENTIALS",
            message: "Backend URLs with embedded credentials are not allowed.".into(),
        });
    }

    let host = parsed.host_str().ok_or_else(|| NetRailError::InvalidBackendUrl {
        code: "BACKEND_URL_NO_HOST",
        message: "Backend URL must include a host.".into(),
    })?;
    block_backend_host(host)?;

    Ok(trimmed.to_string())
}

fn block_backend_host(host: &str) -> NetRailResult<()> {
    let host_lower = host.to_lowercase();

    if host_lower.ends_with(".nip.io")
        || host_lower.ends_with(".sslip.io")
        || host_lower.ends_with(".xip.io")
    {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_DNS_REBINDING",
            message: "DNS rebinding hostnames are not allowed in backend URLs.".into(),
        });
    }

    if let Ok(ip) = host_lower.parse::<IpAddr>() {
        if is_cloud_metadata_ip(ip) {
            return Err(NetRailError::InvalidBackendUrl {
                code: "BACKEND_URL_CLOUD_METADATA",
                message: "Cloud metadata addresses cannot be used as backend URLs.".into(),
            });
        }
        let link_local = match ip {
            IpAddr::V4(v4) => v4.is_link_local(),
            IpAddr::V6(v6) => v6.is_unicast_link_local(),
        };
        if ip.is_unspecified() || link_local {
            return Err(NetRailError::InvalidBackendUrl {
                code: "BACKEND_URL_LINK_LOCAL",
                message: "Unspecified or link-local addresses cannot be used as backend URLs."
                    .into(),
            });
        }
    }

    Ok(())
}

fn is_cloud_metadata_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.octets() == [169, 254, 169, 254],
        IpAddr::V6(v6) => v6.segments() == [0xfd00, 0xec2, 0, 0, 0, 0, 0, 0],
    }
}

pub const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https() {
        assert_eq!(
            validate_open_url("https://example.com/path").unwrap(),
            "https://example.com/path"
        );
    }

    #[test]
    fn rejects_localhost() {
        assert!(validate_open_url("http://127.0.0.1:8080/admin").is_err());
    }

    #[test]
    fn rejects_nip_io() {
        assert!(validate_open_url("http://127.0.0.1.nip.io/").is_err());
    }

    #[test]
    fn unwraps_ddg_redirect_and_blocks_inner_localhost() {
        let ddg = "https://duckduckgo.com/l/?uddg=http%3A%2F%2F127.0.0.1%2Fapi";
        assert!(validate_open_url(ddg).is_err());
    }

    #[test]
    fn unwraps_ddg_redirect_to_safe_url() {
        let ddg = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org%2F";
        assert_eq!(
            validate_open_url(ddg).unwrap(),
            "https://rust-lang.org/"
        );
    }

    #[test]
    fn allows_localhost_searxng_url() {
        assert_eq!(
            validate_backend_url("http://127.0.0.1:8080").unwrap(),
            "http://127.0.0.1:8080"
        );
    }

    #[test]
    fn rejects_metadata_backend_url() {
        assert!(validate_backend_url("http://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn rejects_nip_io_backend_url() {
        assert!(validate_backend_url("http://127.0.0.1.nip.io/").is_err());
    }

    #[test]
    fn open_url_errors_have_stable_codes() {
        let err = validate_open_url("http://127.0.0.1/").unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }
}