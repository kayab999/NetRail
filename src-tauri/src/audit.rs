//! Optional local audit log for mutating / sensitive API actions.
//! Enable with `NETRAIL_AUDIT_LOG=1` or set `NETRAIL_AUDIT_LOG_PATH`.
//!
//! Rotation (A5): the active file is capped at `NETRAIL_AUDIT_MAX_BYTES`
//! (default 10 MiB); on overflow it is rotated to `<path>.1`, shifting older
//! generations up to `NETRAIL_AUDIT_MAX_FILES` (default 3). Set max files to
//! 0 to disable rotation.

use chrono::Utc;
use parking_lot::Mutex;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_MAX_FILES: u32 = 3;

static AUDIT: Mutex<Option<PathBuf>> = Mutex::new(None);

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
    let mut guard = AUDIT.lock();
    if guard.is_none() {
        *guard = resolve_path();
    }
    guard.clone()
}

fn rotation_limits() -> (u64, u32) {
    let max_bytes = env::var("NETRAIL_AUDIT_MAX_BYTES")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_MAX_BYTES);
    let max_files = env::var("NETRAIL_AUDIT_MAX_FILES")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_MAX_FILES);
    (max_bytes, max_files)
}

fn rotate_if_needed(path: &Path, max_bytes: u64, max_files: u32) {
    if max_files == 0 || max_bytes == 0 {
        return;
    }
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if meta.len() < max_bytes {
        return;
    }
    for i in (1..max_files).rev() {
        let src = PathBuf::from(format!("{}.{}", path.display(), i));
        let dst = PathBuf::from(format!("{}.{}", path.display(), i + 1));
        if dst.exists() {
            let _ = std::fs::remove_file(&dst);
        }
        let _ = std::fs::rename(&src, &dst);
    }
    let first = PathBuf::from(format!("{}.1", path.display()));
    if first.exists() {
        let _ = std::fs::remove_file(&first);
    }
    let _ = std::fs::rename(path, &first);
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
    let (max_bytes, max_files) = rotation_limits();
    rotate_if_needed(&path, max_bytes, max_files);
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

#[cfg(test)]
pub fn reset_for_tests() {
    *AUDIT.lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn rotates_audit_log_at_max_bytes() {
        let dir = TempDir::new().unwrap();
        std::env::set_var("NETRAIL_AUDIT_LOG_PATH", dir.path().join("audit.log"));
        std::env::set_var("NETRAIL_AUDIT_MAX_BYTES", "512");
        std::env::set_var("NETRAIL_AUDIT_MAX_FILES", "2");
        reset_for_tests();

        for i in 0..10 {
            log_event("test.action", serde_json::json!({ "i": i }));
        }
        reset_for_tests();

        let rotated = dir.path().join("audit.log.1");
        assert!(rotated.exists(), "expected a rotated audit generation");
        assert!(
            std::fs::metadata(dir.path().join("audit.log"))
                .map(|m| m.len() < 512)
                .unwrap_or(false),
            "active audit file must stay under the cap after rotation"
        );

        std::env::remove_var("NETRAIL_AUDIT_LOG_PATH");
        std::env::remove_var("NETRAIL_AUDIT_MAX_BYTES");
        std::env::remove_var("NETRAIL_AUDIT_MAX_FILES");
        reset_for_tests();
    }

    #[test]
    #[serial]
    fn zero_max_files_disables_rotation() {
        let dir = TempDir::new().unwrap();
        std::env::set_var("NETRAIL_AUDIT_LOG_PATH", dir.path().join("audit.log"));
        std::env::set_var("NETRAIL_AUDIT_MAX_BYTES", "64");
        std::env::set_var("NETRAIL_AUDIT_MAX_FILES", "0");
        reset_for_tests();

        for i in 0..10 {
            log_event("test.action", serde_json::json!({ "i": i }));
        }
        reset_for_tests();

        assert!(
            !dir.path().join("audit.log.1").exists(),
            "rotation must not run when NETRAIL_AUDIT_MAX_FILES=0"
        );

        std::env::remove_var("NETRAIL_AUDIT_LOG_PATH");
        std::env::remove_var("NETRAIL_AUDIT_MAX_BYTES");
        std::env::remove_var("NETRAIL_AUDIT_MAX_FILES");
        reset_for_tests();
    }

    #[test]
    #[serial]
    fn external_rotation_while_writing_does_not_lose_entries() {
        // logrotate-style external rotation: the active file is moved away
        // while the app keeps writing. Every append must recreate the file and
        // no JSON line may be lost; log_event never panics.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("audit.log");
        std::env::set_var("NETRAIL_AUDIT_LOG_PATH", &path);
        std::env::set_var("NETRAIL_AUDIT_MAX_BYTES", "1073741824"); // effectively off
        std::env::set_var("NETRAIL_AUDIT_MAX_FILES", "1");
        reset_for_tests();

        for i in 0..3 {
            log_event("pre.rotation", serde_json::json!({ "i": i }));
        }
        let rotated = dir.path().join("audit.log.rotated");
        std::fs::rename(&path, &rotated).expect("external move of audit.log");
        for i in 0..3 {
            log_event("post.rotation", serde_json::json!({ "i": i }));
        }
        reset_for_tests();

        let count_lines = |p: &PathBuf| {
            std::fs::read_to_string(p)
                .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0)
        };
        assert_eq!(count_lines(&rotated), 3, "pre-rotation entries must survive");
        assert_eq!(count_lines(&path), 3, "post-rotation entries must land in a fresh file");
        assert_eq!(count_lines(&rotated) + count_lines(&path), 6);

        std::env::remove_var("NETRAIL_AUDIT_LOG_PATH");
        std::env::remove_var("NETRAIL_AUDIT_MAX_BYTES");
        std::env::remove_var("NETRAIL_AUDIT_MAX_FILES");
        reset_for_tests();
    }
}
