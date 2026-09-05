//! Single-API-instance guard (S1).
//!
//! The desktop shell and every headless `netrail-api` share one exclusive
//! resource: `127.0.0.1:7421`. The old desktop path spawned `server::start()`
//! and only logged bind failures, so the webview could silently attach to a
//! *foreign* process (other DB, other Fernet key, other stack) or sit dead
//! with a live tray.
//!
//! Policy (attach-explicit):
//! - Probe `GET /api/health` synchronously (no async runtime needed in
//!   Tauri `setup`) before spawning our own server.
//! - Empty port → we bind ourselves.
//! - Valid NetRail fingerprint (`status:"ok"` + `version` + `api_contract`)
//!   → attach to the existing instance, do NOT bind a second server.
//! - Anything else answering on 7421 → fail fast (`exit(1)`), never hijack.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::config::{HOST, PORT};

const PROBE_TIMEOUT: Duration = Duration::from_millis(700);
const MAX_BODY: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthFingerprint {
    /// Raw `/api/health` body (truncated) for logs.
    pub body_snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// Nothing listening — safe to bind.
    Empty,
    /// A real NetRail API answered — attach, do not bind.
    NetRail(HealthFingerprint),
    /// Something else answered — must NOT attach silently.
    Foreign(String),
}

/// True when an `/api/health` body carries the NetRail fingerprint.
/// Opaque but stable: `status:"ok"` plus `version` and `api_contract` keys
/// (see `server::health`). Whitespace-tolerant substring check — no JSON
/// parse needed in the sync probe path.
pub fn is_netrail_body(body: &str) -> bool {
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("\"status\":\"ok\"")
        && compact.contains("\"version\"")
        && compact.contains("\"api_contract\"")
}

/// Synchronous probe of the loopback API. Safe to call from Tauri `setup`
/// (no runtime nesting, std only).
pub fn probe_existing() -> ProbeOutcome {
    let addr: SocketAddr = match format!("{HOST}:{PORT}").parse() {
        Ok(a) => a,
        Err(_) => return ProbeOutcome::Empty,
    };
    let mut stream = match TcpStream::connect_timeout(&addr, PROBE_TIMEOUT) {
        Ok(s) => s,
        Err(_) => return ProbeOutcome::Empty,
    };
    let _ = stream.set_read_timeout(Some(PROBE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(PROBE_TIMEOUT));
    let req = format!("GET /api/health HTTP/1.0\r\nHost: {HOST}:{PORT}\r\nConnection: close\r\n\r\n");
    if stream.write_all(req.as_bytes()).is_err() {
        return ProbeOutcome::Empty;
    }
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > MAX_BODY + 1024 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        return ProbeOutcome::Empty;
    }
    let text = String::from_utf8_lossy(&buf);
    let (head, body) = match text.split_once("\r\n\r\n") {
        Some((h, b)) => (h, b),
        None => match text.split_once("\n\n") {
            Some((h, b)) => (h, b),
            None => ("", text.as_ref()),
        },
    };
    let status_ok = head.contains(" 200 ");
    if !status_ok {
        return ProbeOutcome::Foreign(format!(
            "non-200 from :{PORT} ({})",
            head.lines().next().unwrap_or("?")
        ));
    }
    if is_netrail_body(body) {
        ProbeOutcome::NetRail(HealthFingerprint {
            body_snippet: body.chars().take(300).collect(),
        })
    } else {
        ProbeOutcome::Foreign("200 without NetRail fingerprint".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn fingerprint_requires_all_three_keys() {
        assert!(is_netrail_body(
            r#"{"status":"ok","version":"1.6.6","api_contract":"1.4"}"#
        ));
        assert!(!is_netrail_body(r#"{"status":"ok","version":"1.6.6"}"#));
        assert!(!is_netrail_body(r#"{"status":"ok"}"#));
        assert!(!is_netrail_body("<html>hello</html>"));
    }

    fn spawn_stub(body: &'static str, status: &'static str) -> u16 {
        // Bind an ephemeral port stub; the probe targets PORT via config, so
        // these tests exercise the classifier directly plus a live round-trip
        // through a manual TcpStream against the stub.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut tmp = [0u8; 1024];
                let _ = s.read(&mut tmp);
                let resp = format!(
                    "HTTP/1.0 {status}\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        port
    }

    fn probe_port(port: u16) -> ProbeOutcome {
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let mut stream = TcpStream::connect_timeout(&addr, PROBE_TIMEOUT).unwrap();
        stream
            .set_read_timeout(Some(PROBE_TIMEOUT))
            .unwrap();
        stream
            .write_all(b"GET /api/health HTTP/1.0\r\nHost: x\r\n\r\n")
            .unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }
        let text = String::from_utf8_lossy(&buf);
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        assert!(head.contains(" 200 "));
        if is_netrail_body(body) {
            ProbeOutcome::NetRail(HealthFingerprint {
                body_snippet: body.chars().take(300).collect(),
            })
        } else {
            ProbeOutcome::Foreign("200 without NetRail fingerprint".into())
        }
    }

    #[test]
    fn stub_netrail_body_classifies_attach() {
        let body = r#"{"status":"ok","version":"1.6.6","api_contract":"1.4"}"#;
        let port = spawn_stub(body, "200 OK");
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(matches!(probe_port(port), ProbeOutcome::NetRail(_)));
    }

    #[test]
    fn stub_foreign_body_classifies_abort() {
        let port = spawn_stub("<html>other app</html>", "200 OK");
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(matches!(probe_port(port), ProbeOutcome::Foreign(_)));
    }
}
