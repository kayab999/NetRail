use crate::error::{NetRailError, NetRailResult};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::Url;

/// Base registrable hosts; subdomains (www., r., …) match via suffix.
/// Keep in sync with `url_resolve::DDG_HOSTS` and Python `netrail.security`.
const DDG_HOSTS: &[&str] = &["duckduckgo.com", "duck.com"];
const DNS_REBINDING_HELPERS: &[&str] = &["nip.io", "sslip.io", "xip.io", "localtest.me"];
/// Cloud instance-metadata hostnames (not IP literals). Keep in sync with Python.
const CLOUD_METADATA_HOSTS: &[&str] = &[
    "metadata.google.internal",
    "metadata",
    "instance-data",
];
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
        let host_norm = host_lower.trim_end_matches('.');
        if is_ddg_host(host_norm) {
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
        return Some(effective_ip(ip));
    }
    parse_browser_ipv4(host).map(IpAddr::V4)
}

/// Decode IPv4 embedded in IPv6 forms that carry a real IPv4 in the last
/// 32 bits: IPv4-mapped (`::ffff:a.b.c.d`, via `to_ipv4_mapped`), the RFC
/// 6052 NAT64 well-known prefix (`64:ff9b::/96`), the IPv4-compatible
/// mapped range (`::ffff:0:0/96`, i.e. `::ffff:0:a.b.c.d`) and the
/// deprecated IPv4-compatible range (`::/96`, i.e. `::a.b.c.d`).
fn decode_embedded_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return Some(v4);
    }
    let seg = ip.segments();
    let last = u32::from_be_bytes(
        ip.octets()[12..16]
            .try_into()
            .expect("octets()[12..16] is four bytes"),
    );
    let v4 = Ipv4Addr::from(last);
    let nat64_wkp = seg[0] == 0x0064 && seg[1] == 0xff9b && seg[2..6].iter().all(|&s| s == 0);
    let compat_mapped = seg[0] == 0 && seg[1] == 0 && seg[2] == 0 && seg[3] == 0
        && seg[4] == 0xffff && seg[5] == 0;
    let ipv4_compatible = seg[0..6].iter().all(|&s| s == 0) && ip != Ipv6Addr::LOCALHOST;
    if nat64_wkp || compat_mapped || ipv4_compatible {
        return Some(v4);
    }
    None
}

/// Unmap embedded-IPv4 IPv6 forms so loopback/private checks apply to the
/// embedded IPv4 (SSRF).
fn effective_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => decode_embedded_v4(v6)
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        other => other,
    }
}

fn is_non_public_v4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_link_local()
        || ip.is_private()
        || ip.is_broadcast()
        || ip.is_multicast()
        || matches!(ip.octets()[0], 0 | 240..=255)
        || is_canonical_reserved_v4(ip)
}

/// Canonical URL-policy table (A-10, 2026-08-10): IANA-registered ranges that
/// are never globally routable and therefore never valid browser targets, kept
/// explicit so Rust and Python classify identically regardless of stdlib
/// version (`Ipv4Addr::is_shared` is unstable, so CGNAT is explicit here;
/// Python's `is_private`/`is_reserved` only gained these ranges in 3.13):
///   * 100.64.0.0/10         CGNAT/shared (RFC 6598) — explicit, is_shared unstable
///   * 192.0.0.0/24          IETF protocol assignments
///   * 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24  TEST-NET documentation
///   * 198.18.0.0/15         benchmarking (RFC 2544)
fn is_canonical_reserved_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    (o[0] == 100 && (o[1] & 0xC0) == 0x40) // 100.64.0.0/10
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && (o[1] & 0xfe) == 0x12)
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
}

fn is_non_public_v6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_unicast_link_local()
        || ip.is_unique_local()
        || ip.is_multicast()
        || is_canonical_reserved_v6(ip)
}

/// Canonical URL-policy IPv6 table (A-10, 2026-08-10): the union of Python's
/// `is_reserved` (RFC 4291 "reserved for future use" routing-type schema)
/// and `is_private` (IANA ipv6-special-registry "not globally reachable"):
/// RFC 4291: `::/8`, `100::/8`, `200::/7`, `400::/6`, `800::/5`, `1000::/4`,
/// `4000::/3`, `6000::/3`, `8000::/3`, `a000::/3`, `c000::/3`, `e000::/4`,
/// `f000::/5`, `f800::/6`, `fe00::/9`.
/// IANA private: 2001::/23 except ORCHIDv2 2001:20::/28, 2001:db8::/32,
/// 2002::/16, 3fff::/20, 100::/64, fc00::/7.
/// `::/8` also covers the NAT64 well-known prefix 64:ff9b::/96; embedded-IPv4
/// forms are unmapped to their IPv4 before this check (SSRF), so only
/// non-decodable members land here.
fn is_canonical_reserved_v6(ip: Ipv6Addr) -> bool {
    let b0 = ip.octets()[0];
    let s = ip.segments();
    let iana_private = (s[0] == 0x2001
        && !(s[1] == 0x0020 && (s[2] & 0xF000) == 0) // ORCHIDv2 2001:20::/28
        && s[1] <= 0x007F) // 2001::/23
        || (s[0] == 0x2001 && s[1] == 0x0DB8) // 2001:db8::/32 (0x0DB8 > 0x007F)
        || (s[0] == 0x2002) // 2002::/16
        || (s[0] == 0x3FFF && s[1] <= 0x0FFF) // 3fff::/20
        || (s[0] == 0x0100 && s[1] == 0 && s[2] == 0 && s[3] == 0) // 100::/64
        || (s[0] & 0xFE00) == 0xFC00; // fc00::/7
    let rfc4291 = b0 == 0x00 // ::/8
        || b0 == 0x01 // 100::/8
        || (b0 & 0xFE) == 0x02 // 200::/7
        || (b0 & 0xFC) == 0x04 // 400::/6
        || (b0 & 0xF8) == 0x08 // 800::/5
        || (b0 & 0xF0) == 0x10 // 1000::/4
        || (b0 & 0xE0) == 0x40 // 4000::/3
        || (b0 & 0xE0) == 0x60 // 6000::/3
        || (b0 & 0xE0) == 0x80 // 8000::/3
        || (b0 & 0xE0) == 0xA0 // a000::/3
        || (b0 & 0xE0) == 0xC0 // c000::/3 (fc00::/7 ULA matched by iana_private)
        || (b0 & 0xF0) == 0xE0 // e000::/4
        || (b0 & 0xF8) == 0xF0 // f000::/5
        || (b0 & 0xFC) == 0xF8 // f800::/6
        || (b0 == 0xFE && ip.octets()[1] < 0x80); // fe00::/9 (fe80::/10 link-local first)
    iana_private || rfc4291
}

fn is_cloud_metadata_host(host: &str) -> bool {
    CLOUD_METADATA_HOSTS
        .iter()
        .any(|&h| host == h || host.ends_with(&format!(".{h}")))
}

pub fn normalize_host(host: &str) -> String {
    host.trim_end_matches('.').to_lowercase()
}

fn block_unsafe_host(host: &str) -> NetRailResult<()> {
    let host_lower = normalize_host(host);

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

    if is_cloud_metadata_host(&host_lower) {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_CLOUD_METADATA",
            message: "Cloud metadata hostnames cannot be opened from search results.".into(),
        });
    }

    if let Some(ip) = parse_host_ip(&host_lower) {
        block_ip(ip)?;
    }

    Ok(())
}

fn block_ip(ip: IpAddr) -> NetRailResult<()> {
    match ip {
        IpAddr::V4(v4) if v4.is_loopback() || v4.is_unspecified() => {
            Err(NetRailError::InvalidOpenUrl {
                code: "OPEN_URL_LOCALHOST",
                message: "Localhost URLs cannot be opened from search results.".into(),
            })
        }
        IpAddr::V6(v6) if v6.is_loopback() || v6.is_unspecified() => {
            Err(NetRailError::InvalidOpenUrl {
                code: "OPEN_URL_LOCALHOST",
                message: "Localhost URLs cannot be opened from search results.".into(),
            })
        }
        IpAddr::V4(v4) if v4.is_link_local() => Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_LINK_LOCAL",
            message: "Local or link-local IP addresses cannot be opened from search results."
                .into(),
        }),
        IpAddr::V6(v6) if v6.is_unicast_link_local() => Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_LINK_LOCAL",
            message: "Local or link-local IP addresses cannot be opened from search results."
                .into(),
        }),
        IpAddr::V4(v4) if is_non_public_v4(v4) => Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_PRIVATE",
            message: "Private or non-public IP addresses cannot be opened from search results."
                .into(),
        }),
        IpAddr::V6(v6) if is_non_public_v6(v6) => Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_PRIVATE",
            message: "Private or non-public IP addresses cannot be opened from search results."
                .into(),
        }),
        _ => Ok(()),
    }
}

/// Resolve a hostname to IP addresses via the system resolver. Returns an
/// empty list on failure (NXDOMAIN, no network, …).
pub fn resolve_host_ips(host: &str) -> Vec<IpAddr> {
    use std::net::ToSocketAddrs;
    (host, 0)
        .to_socket_addrs()
        .map(|addrs| addrs.map(|addr| addr.ip()).collect())
        .unwrap_or_default()
}

/// A15: reject hostnames that resolve to non-public IPs. Empty resolution
/// fails closed — if the system resolver cannot answer, the browser could
/// not open the URL anyway.
pub fn check_resolved_host(host: &str, ips: &[IpAddr]) -> NetRailResult<()> {
    if ips.is_empty() {
        return Err(NetRailError::InvalidOpenUrl {
            code: "OPEN_URL_DNS_UNRESOLVABLE",
            message: format!("Could not resolve host {host}."),
        });
    }
    for ip in ips {
        block_ip(*ip)?;
    }
    Ok(())
}

/// A15: pin a previously validated open URL to its current DNS answers
/// before the browser is spawned. IP-literal hosts were already checked by
/// `validate_open_url`; only hostnames are resolved. `resolve` is injectable
/// for tests.
pub fn pin_open_host(safe_url: &str, resolve: impl Fn(&str) -> Vec<IpAddr>) -> NetRailResult<()> {
    let parsed = Url::parse(safe_url).map_err(|_| NetRailError::InvalidOpenUrl {
        code: "OPEN_URL_INVALID",
        message: "Invalid URL.".into(),
    })?;
    if let Some(host) = parsed.host_str() {
        let host_lower = host.trim_end_matches('.').to_lowercase();
        if parse_host_ip(&host_lower).is_none() {
            check_resolved_host(&host_lower, &resolve(&host_lower))?;
        }
    }
    Ok(())
}

/// Validate a user-configured backend URL (e.g. SearXNG). Localhost and private
/// LAN hosts are allowed by default; cloud metadata, rebinding hostnames, and
/// link-local addresses are blocked. Pass `strict = true` to also reject
/// loopback / private IPs (cloud-safe / multi-tenant style).
pub fn validate_backend_url(raw: &str) -> NetRailResult<String> {
    validate_backend_url_with_options(raw, false)
}

pub fn validate_backend_url_with_options(raw: &str, strict: bool) -> NetRailResult<String> {
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
    block_backend_host(host, strict)?;

    Ok(trimmed.to_string())
}

/// A-05: fetch-time re-validation of a backend URL (parity anchor for the
/// Python `check_backend_fetch_url`). Hostnames that will actually be fetched
/// are resolved and the `block_backend_host` rules are applied to every
/// resolved address: cloud metadata and unspecified/link-local are always
/// rejected, other non-public ranges only when `strict`. Empty resolution
/// fails closed (BACKEND_URL_DNS_UNRESOLVABLE). IP-literal backends are
/// classified without DNS. `resolve` is injectable for tests; the desktop
/// shell validates config offline at save time and the SearXNG fetch lives in
/// the Python process, so this helper is the semantic anchor any future Rust
/// fetch surface must call before issuing a request.
pub fn check_backend_fetch_url(
    raw: &str,
    strict: bool,
    resolve: impl Fn(&str) -> Vec<IpAddr>,
) -> NetRailResult<()> {
    let trimmed = raw.trim();
    let parsed = Url::parse(trimmed).map_err(|_| NetRailError::InvalidBackendUrl {
        code: "BACKEND_URL_INVALID",
        message: "Invalid backend URL.".into(),
    })?;
    let host = parsed.host_str().ok_or_else(|| NetRailError::InvalidBackendUrl {
        code: "BACKEND_URL_INVALID",
        message: "Invalid backend URL.".into(),
    })?;
    let host_lower = host.trim_end_matches('.').to_lowercase();

    block_backend_host(&host_lower, strict)?;
    if parse_host_ip(&host_lower).is_some() {
        return Ok(());
    }

    let ips = resolve(&host_lower);
    if ips.is_empty() {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_DNS_UNRESOLVABLE",
            message: format!("Backend host {host} does not resolve."),
        });
    }
    for ip in ips {
        block_backend_host(&ip.to_string(), strict)?;
    }
    Ok(())
}

fn block_backend_host(host: &str, strict: bool) -> NetRailResult<()> {
    let host_lower = host.trim_end_matches('.').to_lowercase();

    if is_dns_rebinding_helper(&host_lower) {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_DNS_REBINDING",
            message: "DNS rebinding hostnames are not allowed in backend URLs.".into(),
        });
    }

    if is_cloud_metadata_host(&host_lower) {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_CLOUD_METADATA",
            message: "Cloud metadata addresses cannot be used as backend URLs.".into(),
        });
    }

    if strict
        && matches!(
            host_lower.as_str(),
            "localhost" | "127.0.0.1" | "::1" | "0.0.0.0" | "[::1]"
        )
    {
        return Err(NetRailError::InvalidBackendUrl {
            code: "BACKEND_URL_STRICT_PRIVATE",
            message: "strict_backend_urls rejects localhost backends.".into(),
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
        if strict {
            let private = match ip {
                IpAddr::V4(v4) => is_non_public_v4(v4) || v4.is_loopback(),
                IpAddr::V6(v6) => is_non_public_v6(v6) || v6.is_loopback(),
            };
            if private {
                return Err(NetRailError::InvalidBackendUrl {
                    code: "BACKEND_URL_STRICT_PRIVATE",
                    message: "strict_backend_urls rejects private/loopback backend hosts.".into(),
                });
            }
        }
    }

    Ok(())
}

fn is_cloud_metadata_ip(ip: IpAddr) -> bool {
    match effective_ip(ip) {
        IpAddr::V4(v4) => v4.octets() == [169, 254, 169, 254],
        // AWS IMDS IPv6: fd00:ec2::254
        IpAddr::V6(v6) => v6.segments() == [0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254],
    }
}

// No `upgrade-insecure-requests`: WebKitGTK (the desktop webview) upgrades
// http subresources to https — including 127.0.0.1 — and a loopback-only,
// plain-http server then fails every static/API subresource load (white UI).
// Chromium exempts localhost; WebKit does not (verified on 2.50.4, 2026-08-10).
pub const CSP: &str = "default-src 'self'; script-src 'self' 'sha256-aN9klVksJOk4OThOcI2OMlo7DsWPc+W7cPY4E+ODbD8='; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_open_host_blocks_loopback_resolution() {
        let fake = |_: &str| vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
        let err = pin_open_host("http://internal.corp/", fake).unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

    #[test]
    fn pin_open_host_blocks_private_resolution() {
        let fake = |_: &str| vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))];
        let err = pin_open_host("https://evil.example/", fake).unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_PRIVATE");
    }

    #[test]
    fn pin_open_host_blocks_link_local_resolution() {
        let fake = |_: &str| vec![IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))];
        let err = pin_open_host("http://metadata-helper.example/", fake).unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LINK_LOCAL");
    }

    #[test]
    fn pin_open_host_allows_public_resolution() {
        let fake = |_: &str| vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))];
        assert!(pin_open_host("https://example.org/", fake).is_ok());
    }

    #[test]
    fn pin_open_host_fails_closed_on_unresolvable_host() {
        let fake = |_: &str| vec![];
        let err = pin_open_host("https://nxdomain.invalid/", fake).unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_DNS_UNRESOLVABLE");
    }

    #[test]
    fn pin_open_host_skips_ip_literals() {
        let fake = |h: &str| panic!("resolver must not run for IP literals, got {h}");
        assert!(pin_open_host("https://93.184.216.34/", fake).is_ok());
    }

    #[test]
    fn pin_open_host_blocks_any_non_public_answer() {
        let fake = |_: &str| vec![IpAddr::V6(Ipv6Addr::LOCALHOST), IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))];
        let err = pin_open_host("https://dual.example/", fake).unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

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
            let strict = case["strict"].as_bool().unwrap_or(false);
            let check = |u: &str, s: bool| -> NetRailResult<String> {
                if let Some(ips) = case["resolved_ips"].as_array() {
                    let addr: Vec<IpAddr> = ips
                        .iter()
                        .map(|v| v.as_str().expect("resolved_ips string").parse().expect("ip"))
                        .collect();
                    // A-05 fetch-time vectors: the hostname resolves to the
                    // fixture addresses; classification must match save-time.
                    check_backend_fetch_url(u, s, |_h| addr.clone())?;
                    Ok(u.to_string())
                } else {
                    validate_backend_url_with_options(u, s)
                }
            };
            match expect {
                "allow" => {
                    let got = check(url, strict)
                        .unwrap_or_else(|e| panic!("backend_url {id}: expected allow, got {e:?}"));
                    if let Some(normalized) = case["normalized"].as_str() {
                        assert_eq!(got, normalized, "backend_url {id}");
                    }
                }
                "block" => {
                    let err = check(url, strict).expect_err(&format!("backend_url {id}"));
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
    fn backend_fetch_blocks_metadata_resolution_non_strict() {
        let fake = |_: &str| vec![IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))];
        let err = check_backend_fetch_url("http://company-searxng.internal:8080", false, fake)
            .unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_CLOUD_METADATA");
    }

    #[test]
    fn backend_fetch_blocks_aws_metadata_ipv6_resolution() {
        let fake = |_: &str| vec!["fd00:ec2::254".parse().unwrap()];
        let err = check_backend_fetch_url("http://searxng.example:8080", false, fake).unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_CLOUD_METADATA");
    }

    #[test]
    fn backend_fetch_blocks_link_local_resolution_non_strict() {
        let fake = |_: &str| vec!["fe80::1".parse().unwrap()];
        let err = check_backend_fetch_url("http://searxng.lan:8080", false, fake).unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_LINK_LOCAL");
    }

    #[test]
    fn backend_fetch_blocks_unspecified_resolution_non_strict() {
        let fake = |_: &str| vec![IpAddr::V6(Ipv6Addr::UNSPECIFIED)];
        let err = check_backend_fetch_url("http://searxng.lan:8080", false, fake).unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_LINK_LOCAL");
    }

    #[test]
    fn backend_fetch_fails_closed_on_empty_resolution() {
        let fake = |_: &str| vec![];
        let err = check_backend_fetch_url("http://gone.invalid:8080", false, fake).unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_DNS_UNRESOLVABLE");
    }

    #[test]
    fn backend_fetch_blocks_private_resolution_strict_only() {
        let fake = |_: &str| vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))];
        let err = check_backend_fetch_url("http://searxng.lan:8080", true, fake).unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_STRICT_PRIVATE");
        assert!(check_backend_fetch_url("http://searxng.lan:8080", false, fake).is_ok());
    }

    #[test]
    fn backend_fetch_allows_public_resolution_strict() {
        let fake = |_: &str| vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))];
        assert!(check_backend_fetch_url("http://searxng.example.com:8080", true, fake).is_ok());
    }

    #[test]
    fn backend_fetch_never_resolves_ip_literals() {
        let fake = |h: &str| panic!("resolver must not run for IP literals, got {h}");
        assert!(check_backend_fetch_url("http://127.0.0.1:8080", false, fake).is_ok());
        assert!(check_backend_fetch_url("http://[fd00::1]:8080", false, fake).is_ok());
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

    #[test]
    fn rejects_ipv4_mapped_loopback() {
        let err = validate_open_url("http://[::ffff:127.0.0.1]/").unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_LOCALHOST");
    }

    #[test]
    fn rejects_ipv4_mapped_private() {
        let err = validate_open_url("http://[::ffff:c0a8:101]/").unwrap_err();
        assert_eq!(err.error_code(), "OPEN_URL_PRIVATE");
    }

    #[test]
    fn rejects_aws_ipv6_metadata_backend() {
        let err = validate_backend_url("http://[fd00:ec2::254]/").unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_CLOUD_METADATA");
    }

    #[test]
    fn rejects_metadata_hostname_open_and_backend() {
        let open_err = validate_open_url("http://metadata.google.internal/").unwrap_err();
        assert_eq!(open_err.error_code(), "OPEN_URL_CLOUD_METADATA");
        let back_err = validate_backend_url("http://metadata.google.internal/").unwrap_err();
        assert_eq!(back_err.error_code(), "BACKEND_URL_CLOUD_METADATA");
    }

    #[test]
    fn strict_backend_rejects_localhost() {
        assert!(validate_backend_url("http://127.0.0.1:8080").is_ok());
        let err = validate_backend_url_with_options("http://127.0.0.1:8080", true).unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_STRICT_PRIVATE");
    }

    // --- S1: Invariant & Property Tests ---

    #[test]
    fn property_normalize_host_is_idempotent() {
        let sample_hosts = [
            "127.0.0.1.",
            "DUCKDUCKGO.COM..",
            "192.168.1.1",
            "metadata.google.internal.",
            "EXAMPLE.COM%2Fpath",
            "127.0.0.1",
            "localhost",
            "[::1]",
        ];
        for host in sample_hosts {
            let once = normalize_host(host);
            let twice = normalize_host(&once);
            assert_eq!(once, twice, "idempotency failed for host: {host}");
        }
    }

    #[test]
    fn property_parse_browser_ipv4_never_panics_on_arbitrary_input() {
        let fuzz_inputs = [
            "",
            "   ",
            ".",
            "...",
            "0",
            "0x",
            "0xGG",
            "256.256.256.256",
            "127.0.0.1.1",
            "-1",
            "99999999999999999999999999999",
            "0177.0.0.1",
            "0x7f000001",
            "2130706433",
            "127.1",
            "127.0.1",
            "192.168.1.1",
            "../../../etc/passwd",
            "\x00\x01\x02",
        ];
        for input in fuzz_inputs {
            let _ = parse_browser_ipv4(input);
        }
    }


    #[test]
    fn property_pin_open_host_fail_closed_on_empty_dns() {
        let empty_ips: &[std::net::IpAddr] = &[];
        let res = check_resolved_host("example.com", empty_ips);
        assert!(res.is_err(), "Empty DNS resolution must fail closed");
        assert_eq!(res.unwrap_err().error_code(), "OPEN_URL_DNS_UNRESOLVABLE");
    }

    #[test]
    fn property_pin_open_host_blocks_private_dns_answers() {
        let private_ips = [std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 50))];
        let res = check_resolved_host("rebinding.example.com", &private_ips);
        assert!(res.is_err(), "DNS resolving to private IP must be rejected");
        assert_eq!(res.unwrap_err().error_code(), "OPEN_URL_PRIVATE");
    }
}


