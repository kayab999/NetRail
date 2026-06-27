use std::net::IpAddr;
use url::Url;

const DDG_HOSTS: &[&str] = &["duckduckgo.com", "r.duckduckgo.com"];
const MAX_REDIRECT_DEPTH: u8 = 5;

pub fn validate_open_url(raw: &str) -> Result<String, String> {
    validate_open_url_inner(raw.trim(), 0)
}

fn validate_open_url_inner(raw: &str, depth: u8) -> Result<String, String> {
    if depth > MAX_REDIRECT_DEPTH {
        return Err("Too many redirect wrappers.".into());
    }

    let parsed = Url::parse(raw).map_err(|_| "Invalid URL.".to_string())?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("Only http:// and https:// URLs are supported.".into());
    }

    if parsed.username() != "" || parsed.password().is_some() {
        return Err("URLs with embedded credentials are not allowed.".into());
    }

    if let Some(host) = parsed.host_str() {
        let host_lower = host.to_lowercase();
        if is_ddg_host(&host_lower) {
            if let Some((_, uddg)) = parsed.query_pairs().find(|(k, _)| k == "uddg") {
                return validate_open_url_inner(&uddg, depth + 1);
            }
        }
    }

    block_unsafe_host(parsed.host_str().ok_or("URL must include a host.")?)?;

    Ok(raw.to_string())
}

fn is_ddg_host(host: &str) -> bool {
    DDG_HOSTS.iter().any(|&h| host == h || host.ends_with(&format!(".{h}")))
}

fn block_unsafe_host(host: &str) -> Result<(), String> {
    let host_lower = host.to_lowercase();

    if matches!(
        host_lower.as_str(),
        "localhost" | "127.0.0.1" | "::1" | "0.0.0.0" | "[::1]"
    ) {
        return Err("Localhost URLs cannot be opened from search results.".into());
    }

    if host_lower.ends_with(".nip.io")
        || host_lower.ends_with(".sslip.io")
        || host_lower.ends_with(".xip.io")
    {
        return Err("DNS rebinding hostnames cannot be opened from search results.".into());
    }

    if let Ok(ip) = host_lower.parse::<IpAddr>() {
        let link_local = match ip {
            IpAddr::V4(v4) => v4.is_link_local(),
            IpAddr::V6(v6) => v6.is_unicast_link_local(),
        };
        if ip.is_loopback() || ip.is_unspecified() || link_local {
            return Err("Local or link-local IP addresses cannot be opened from search results.".into());
        }
    }

    Ok(())
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
}