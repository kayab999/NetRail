use crate::error::{NetRailError, NetRailResult};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::Url;

/// Base registrable hosts; subdomains (www., r., …) match via suffix.
/// Keep in sync with `url_resolve::DDG_HOSTS` and Python `netrail.security`.
const DDG_HOSTS: &[&str] = &["duckduckgo.com", "duck.com"];
const DNS_REBINDING_HELPERS: &[&str] = &["nip.io", "sslip.io", "xip.io", "localtest.me"];
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
    DDG_HOSTS
        .iter()
        .any(|&h| host == h || host.ends_with(&format!(".{h}")))
}

fn is_dns_rebinding_helper(host: &str) -> bool {
    DNS_REBINDING_HELPERS
        .iter()
        .any(|&d| host == d || host.ends_with(&format!(".{d}")))
}

/// Parse IPv4 the way browsers often do: decimal integer, hex, octal octets,
/// and short forms (`127.1` → `127.0.0.1`).
fn parse_browser_ipv4(host: &str) -> Option<Ipv4Addr> {
    let host = host.trim().trim_matches(|c| c == '[' || c == ']');
    if host.is_empty() {
        return None;
    }

    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Some(ip);
    }

    if !host.contains('.') {
        return parse_u32_loose(host).map(Ipv4Addr::from);
    }

    let parts: Vec<&str> = host.split('.').collect();
    match parts.len() {
        2 => {
            let a = parse_u32_loose(parts[0])?;
            let b = parse_u32_loose(parts[1])?;
            if a > 255 || b > 0x00FF_FFFF {
                return None;
            }
            Some(Ipv4Addr::from((a << 24) | b))
        }
        3 => {
            let a = parse_u32_loose(parts[0])?;
            let b = parse_u32_loose(parts[1])?;
            let c = parse_u32_loose(parts[2])?;
            if a > 255 || b > 255 || c > 0xFFFF {
                return None;
            }
            Some(Ipv4Addr::from((a << 24) | (b << 16) | c))
        }
        4 => {
            let mut octets = [0u8; 4];
            for (i, part) in parts.iter().enumerate() {
                let n = parse_u32_loose(part)?;
                if n > 255 {
                    return None;
                }
                octets[i] = n as u8;
            }
            Some(Ipv4Addr::from(octets))
        }
        _ => None,
    }
}

/// Decimal, `0x` hex, or leading-zero octal (browser-style).
fn parse_u32_loose(raw: &str) -> Option<u32> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(hex) = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
    {
        return u32::from_str_radix(hex, 16).ok();
    }
    if s.len() > 1 && s.starts_with('0') && s.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return u32::from_str_radix(s, 8).ok();
    }
    s.parse::<u32>().ok()
}

fn parse_host_ip(host: &str) -> Option<IpAddr> {
    let host = host.trim().trim_matches(|c| c == '[' || c == ']');
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(ip);
    }
    parse_browser_ipv4(host).map(IpAddr::V4)
}

fn is_non_public_v4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_link_local()
        || ip.is_private()
        || ip.is_broadcast()
        || ip.is_multicast()
        || matches!(ip.octets()[0], 0 | 240..=255)
}

fn is_non_public_v6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_unicast_link_local()
        || ip.is_unique_local()
        || ip.is_multicast()
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

    if is_dns_rebinding_helper(&host_lower) {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_DNS_REBINDING",
            message: "DNS rebinding hostnames cannot be opened from search results.".into(),
        });
    }

    if let Some(ip) = parse_host_ip(&host_lower) {
        match ip {
            IpAddr::V4(v4) if v4.is_loopback() || v4.is_unspecified() => {
                return Err(NetRailError::InvalidOpenUrl {
                    code: "OPEN_URL_LOCALHOST",
                    message: "Localhost URLs cannot be opened from search results.".into(),
                });
            }
            IpAddr::V6(v6) if v6.is_loopback() || v6.is_unspecified() => {
                return Err(NetRailError::InvalidOpenUrl {
                    code: "OPEN_URL_LOCALHOST",
                    message: "Localhost URLs cannot be opened from search results.".into(),
                });
            }
            IpAddr::V4(v4) if v4.is_link_local() => {
                return Err(NetRailError::InvalidOpenUrl {
                    code: "OPEN_URL_LINK_LOCAL",
                    message:
                        "Local or link-local IP addresses cannot be opened from search results."
                            .into(),
                });
            }
            IpAddr::V6(v6) if v6.is_unicast_link_local() => {
                return Err(NetRailError::InvalidOpenUrl {
                    code: "OPEN_URL_LINK_LOCAL",
                    message:
                        "Local or link-local IP addresses cannot be opened from search results."
                            .into(),
                });
            }
            IpAddr::V4(v4) if is_non_public_v4(v4) => {
                return Err(NetRailError::InvalidOpenUrl {
                    code: "OPEN_URL_PRIVATE",
                    message: "Private or non-public IP addresses cannot be opened from search results."
                        .into(),
                });
            }
            IpAddr::V6(v6) if is_non_public_v6(v6) => {
                return Err(NetRailError::InvalidOpenUrl {
                    code: "OPEN_URL_PRIVATE",
                    message: "Private or non-public IP addresses cannot be opened from search results."
                        .into(),
                });
            }
            _ => {}
        }
    }

    Ok(())
}

/// Validate a user-configured backend URL (e.g. SearXNG). Localhost and private
/// LAN hosts are allowed; cloud metadata, rebinding hostnames, and link-local
/// addresses are blocked.
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

    if is_dns_rebinding_helper(&host_lower) {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_DNS_REBINDING",
            message: "DNS rebinding hostnames are not allowed in backend URLs.".into(),
        });
    }

    if let Some(ip) = parse_host_ip(&host_lower) {
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
    fn rejects_rebinding_apex_domains() {
        for url in [
            "http://localtest.me/",
            "http://nip.io/",
            "http://sslip.io/",
            "http://xip.io/",
        ] {
            let err = validate_open_url(url).unwrap_err();
            assert_eq!(err.error_code(), "OPEN_URL_DNS_REBINDING", "{url}");
        }
    }

    #[test]
    fn unwraps_ddg_redirect_and_blocks_inner_localhost() {
        let ddg = "https://duckduckgo.com/l/?uddg=http%3A%2F%2F127.0.0.1%2Fapi";
        assert!(validate_open_url(ddg).is_err());
    }

    #[test]
    fn unwraps_duck_com_redirect_and_blocks_inner_localhost() {
        let ddg = "https://duck.com/l/?uddg=http%3A%2F%2F127.0.0.1%2F";
        let err = validate_open_url(ddg).unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

    #[test]
    fn unwraps_ddg_redirect_to_safe_url() {
        let ddg = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org%2F";
        assert_eq!(validate_open_url(ddg).unwrap(), "https://rust-lang.org/");
    }

    #[test]
    fn golden_url_policy_fixture() {
        let raw = include_str!("../../tests/fixtures/url_policy.json");
        let fixture: serde_json::Value =
            serde_json::from_str(raw).expect("url_policy.json must parse");

        for case in fixture["open_url"].as_array().expect("open_url array") {
            let id = case["id"].as_str().unwrap_or("?");
            let url = case["url"].as_str().expect("url");
            let expect = case["expect"].as_str().expect("expect");
            match expect {
                "allow" => {
                    let got = validate_open_url(url).unwrap_or_else(|e| {
                        panic!("open_url {id}: expected allow, got {e:?}")
                    });
                    if let Some(normalized) = case["normalized"].as_str() {
                        assert_eq!(got, normalized, "open_url {id}");
                    }
                }
                "block" => {
                    let err = validate_open_url(url).expect_err(&format!("open_url {id}"));
                    if let Some(code) = case["code"].as_str() {
                        assert_eq!(err.error_code(), code, "open_url {id}");
                    }
                }
                other => panic!("open_url {id}: bad expect {other}"),
            }
        }

        for case in fixture["backend_url"].as_array().expect("backend_url array") {
            let id = case["id"].as_str().unwrap_or("?");
            let url = case["url"].as_str().expect("url");
            let expect = case["expect"].as_str().expect("expect");
            match expect {
                "allow" => {
                    let got = validate_backend_url(url).unwrap_or_else(|e| {
                        panic!("backend_url {id}: expected allow, got {e:?}")
                    });
                    if let Some(normalized) = case["normalized"].as_str() {
                        assert_eq!(got, normalized, "backend_url {id}");
                    }
                }
                "block" => {
                    let err = validate_backend_url(url).expect_err(&format!("backend_url {id}"));
                    if let Some(code) = case["code"].as_str() {
                        assert_eq!(err.error_code(), code, "backend_url {id}");
                    }
                }
                other => panic!("backend_url {id}: bad expect {other}"),
            }
        }
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

    #[test]
    fn rejects_decimal_encoded_loopback() {
        let err = validate_open_url("http://2130706433/").unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

    #[test]
    fn rejects_hex_encoded_loopback() {
        let err = validate_open_url("http://0x7f000001/").unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

    #[test]
    fn rejects_octal_dotted_loopback() {
        let err = validate_open_url("http://0177.0.0.1/").unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

    #[test]
    fn rejects_short_form_loopback() {
        let err = validate_open_url("http://127.1/").unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

    #[test]
    fn rejects_private_rfc1918() {
        for url in [
            "http://192.168.1.1/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
        ] {
            let err = validate_open_url(url).unwrap_err();
            assert_eq!(err.error_code(), "OPEN_URL_PRIVATE", "{url}");
        }
    }

    #[test]
    fn allows_private_backend_url_for_searxng() {
        assert_eq!(
            validate_backend_url("http://192.168.0.5:8080").unwrap(),
            "http://192.168.0.5:8080"
        );
    }

    #[test]
    fn parse_browser_ipv4_decimal() {
        assert_eq!(
            parse_browser_ipv4("2130706433"),
            Some(Ipv4Addr::new(127, 0, 0, 1))
        );
    }
}
