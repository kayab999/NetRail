//! Optional local audit log for mutating / sensitive API actions.
//! Enable with `NETRAIL_AUDIT_LOG=1` or set `NETRAIL_AUDIT_LOG_PATH`.

use chrono::Utc;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

static AUDIT: OnceCell<Mutex<Option<PathBuf>>> = OnceCell::new();

fn resolve_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("NETRAIL_AUDIT_LOG_PATH") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    let enabled = env::var("NETRAIL_AUDIT_LOG")
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false") && !v.is_empty())
        .unwrap_or(false);
    if !enabled {
        return None;
    }
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("netrail");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("audit.log"))
}

fn path() -> Option<PathBuf> {
    let cell = AUDIT.get_or_init(|| Mutex::new(resolve_path()));
    cell.lock().clone()
}

/// Append one JSON line. Never panics; failures are traced only.
pub fn log_event(action: &str, detail: serde_json::Value) {
    let Some(path) = path() else {
        return;
    };
    let line = serde_json::json!({
        "ts": Utc::now().to_rfc3339(),
        "action": action,
        "detail": detail,
    });
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        tracing::warn!(path = %path.display(), "audit log open failed");
        return;
    };
    if let Ok(raw) = serde_json::to_string(&line) {
        let _ = writeln!(file, "{raw}");
    }
}

pub fn enabled() -> bool {
    path().is_some()
}
