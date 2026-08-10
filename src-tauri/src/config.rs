use crate::error::{NetRailError, NetRailResult};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const VERSION: &str = "1.6.4";
/// Stable API contract version (additive changes only; see docs/AUDIT_ARCH A12).
pub const API_CONTRACT: &str = "1.4";
pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 7421;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub url: Option<String>,
    /// Environment variable name holding the API key — never stored in settings.
    #[serde(default)]
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub browser_id: Option<String>,
    pub private_mode: bool,
    pub max_results: u32,
    #[serde(default = "default_backend_order")]
    pub backend_order: Vec<String>,
    #[serde(default = "default_true")]
    pub ddgs_enabled: bool,
    pub searxng_url: Option<String>,
    #[serde(default)]
    pub brave_enabled: bool,
    #[serde(default = "default_search_strategy")]
    pub search_strategy: String,
    #[serde(default)]
    pub backends: Vec<BackendConfig>,
    pub history_enabled: bool,
    pub history_encrypt: bool,
    pub history_ttl_days: u32,
    /// When true, SearXNG/backend URLs may not use loopback or private IPs.
    #[serde(default)]
    pub strict_backend_urls: bool,
}

fn default_backend_order() -> Vec<String> {
    vec!["searxng".into(), "ddgs".into(), "brave".into()]
}

fn default_search_strategy() -> String {
    "fanout".into()
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            browser_id: None,
            private_mode: false,
            max_results: 25,
            backend_order: default_backend_order(),
            ddgs_enabled: true,
            searxng_url: None,
            brave_enabled: false,
            search_strategy: default_search_strategy(),
            backends: default_backends(),
            history_enabled: true,
            history_encrypt: true,
            history_ttl_days: 90,
            strict_backend_urls: false,
        }
    }
}

pub fn default_backends() -> Vec<BackendConfig> {
    vec![
        BackendConfig {
            id: "searxng".into(),
            enabled: true,
            url: None,
            api_key_env: None,
        },
        BackendConfig {
            id: "ddgs".into(),
            enabled: true,
            url: None,
            api_key_env: None,
        },
        BackendConfig {
            id: "brave".into(),
            enabled: false,
            url: None,
            api_key_env: Some("BRAVE_SEARCH_API_KEY".into()),
        },
    ]
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("netrail")
}

pub fn config_file() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load_settings() -> Settings {
    let path = config_file();
    let mut settings = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Settings>(&raw) {
                Ok(parsed) => parsed,
                Err(err) => {
                    tracing::warn!(
                        path = %path.display(),
                        %err,
                        "settings.json is corrupt; falling back to defaults"
                    );
                    Settings::default()
                }
            },
            Err(err) => {
                tracing::warn!(
                    path = %path.display(),
                    %err,
                    "settings.json unreadable; falling back to defaults"
                );
                Settings::default()
            }
        }
    } else {
        Settings::default()
    };

    apply_env_overrides(&mut settings);
    settings
}

pub fn validate_settings(settings: &Settings) -> NetRailResult<()> {
    use crate::security::validate_backend_url_with_options;

    if settings.max_results < 1 || settings.max_results > 50 {
        return Err(NetRailError::InvalidConfig {
            code: "CONFIG_MAX_RESULTS",
            message: "max_results must be between 1 and 50.".into(),
        });
    }
    if settings.history_ttl_days > 3650 {
        return Err(NetRailError::InvalidConfig {
            code: "CONFIG_HISTORY_TTL",
            message: "history_ttl_days must be at most 3650.".into(),
        });
    }
    if settings.search_strategy != "fanout" && settings.search_strategy != "fallback" {
        return Err(NetRailError::InvalidConfig {
            code: "CONFIG_SEARCH_STRATEGY",
            message: "search_strategy must be 'fanout' or 'fallback'.".into(),
        });
    }
    let strict = settings.strict_backend_urls || strict_backend_urls_from_env();
    if let Some(ref url) = settings.searxng_url {
        validate_backend_url_with_options(url, strict)?;
    }
    for entry in &settings.backends {
        if let Some(ref url) = entry.url {
            validate_backend_url_with_options(url, strict)?;
        }
    }
    Ok(())
}

pub fn strict_backend_urls_from_env() -> bool {
    env::var("NETRAIL_STRICT_BACKEND_URLS")
        .map(|v| parse_bool(&v))
        .unwrap_or(false)
}

/// `NETRAIL_READONLY=1` rejects all mutating API calls (settings PUT,
/// history delete/purge, collections create/add) with 403 READONLY_MODE.
/// Read endpoints (search, open, docs, export) keep working.
pub fn readonly_mode() -> bool {
    env::var("NETRAIL_READONLY")
        .map(|v| parse_bool(&v))
        .unwrap_or(false)
}

pub fn save_settings(settings: &Settings) -> NetRailResult<Settings> {
    validate_settings(settings)?;
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    let target = config_file();
    // Unique per attempt (pid + thread + monotonic counter) so concurrent
    // PUT /api/settings cannot share a temp path (NR-08).
    let tmp_file = {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};
        static SAVE_SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SAVE_SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        dir.join(format!(
            "settings.json.tmp.{}.{}.{}",
            std::process::id(),
            seq,
            nanos
        ))
    };
    let payload = serde_json::to_string_pretty(settings)?;
    if let Err(err) = fs::write(&tmp_file, format!("{payload}\n")) {
        let _ = fs::remove_file(&tmp_file);
        return Err(NetRailError::Internal {
            code: "CONFIG_SAVE_FAILED",
            message: format!("Failed to write settings temp file: {err}"),
        });
    }
    if let Err(err) = fs::rename(&tmp_file, &target) {
        let _ = fs::remove_file(&tmp_file);
        return Err(NetRailError::Internal {
            code: "CONFIG_SAVE_FAILED",
            message: format!("Failed to rename settings file: {err}"),
        });
    }
    Ok(load_settings())
}

fn apply_env_overrides(settings: &mut Settings) {
    if let Ok(raw) = env::var("NETRAIL_STRICT_BACKEND_URLS") {
        settings.strict_backend_urls = parse_bool(&raw);
    }
    if let Ok(url) = env::var("NETRAIL_SEARXNG_URL").or_else(|_| env::var("SEARXNG_URL")) {
        if !url.is_empty() {
            let strict = settings.strict_backend_urls || strict_backend_urls_from_env();
            // Same gate as settings save — never apply metadata/rebinding from env.
            match crate::security::validate_backend_url_with_options(&url, strict) {
                Ok(safe) => settings.searxng_url = Some(safe),
                Err(_) => {
                    // Leave prior settings value; invalid env must not enable a hostile backend.
                    tracing::warn!(
                        "Ignoring invalid NETRAIL_SEARXNG_URL / SEARXNG_URL (failed backend URL policy)"
                    );
                }
            }
        }
    }
    if let Ok(raw) = env::var("NETRAIL_BRAVE_ENABLED") {
        settings.brave_enabled = parse_bool(&raw);
    }
    if env::var("BRAVE_SEARCH_API_KEY")
        .or_else(|_| env::var("NETRAIL_BRAVE_API_KEY"))
        .is_ok()
    {
        settings.brave_enabled = true;
        for backend in &mut settings.backends {
            if backend.id == "brave" {
                backend.enabled = true;
            }
        }
        if !settings.backend_order.iter().any(|b| b == "brave") {
            settings.backend_order.push("brave".into());
        }
    }
    if let Ok(raw) = env::var("NETRAIL_SEARCH_STRATEGY") {
        let lower = raw.to_lowercase();
        if lower == "fanout" || lower == "fallback" {
            settings.search_strategy = lower;
        }
    }
    if let Ok(raw) = env::var("NETRAIL_HISTORY_ENABLED") {
        settings.history_enabled = parse_bool(&raw);
    }
    if let Ok(raw) = env::var("NETRAIL_HISTORY_ENCRYPT") {
        settings.history_encrypt = parse_bool(&raw);
    }
    if let Ok(raw) = env::var("NETRAIL_HISTORY_TTL_DAYS") {
        if let Ok(days) = raw.parse() {
            settings.history_ttl_days = days;
        }
    }
    if let Ok(raw) = env::var("NETRAIL_MAX_RESULTS") {
        if let Ok(max) = raw.parse() {
            settings.max_results = max;
        }
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub fn is_flatpak() -> bool {
    PathBuf::from("/.flatpak-info").exists()
}

/// Resolve the web UI directory at runtime (dev checkout, deb, AppImage, or override).
pub fn static_dir() -> PathBuf {
    use std::sync::OnceLock;
    static RESOLVED: OnceLock<PathBuf> = OnceLock::new();
    RESOLVED
        .get_or_init(resolve_static_dir)
        .clone()
}

fn resolve_static_dir() -> PathBuf {
    if let Ok(dir) = env::var("NETRAIL_STATIC_DIR") {
        let path = PathBuf::from(dir);
        if path.join("index.html").is_file() {
            return path;
        }
        tracing::warn!(
            path = %path.display(),
            "NETRAIL_STATIC_DIR is set but index.html is missing"
        );
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            let candidates = [
                bin_dir.join("../share/netrail/static"),
                bin_dir.join("../../share/netrail/static"),
                // AppImage / some bundles place resources next to the binary.
                bin_dir.join("netrail/static"),
            ];
            for candidate in candidates {
                if candidate.join("index.html").is_file() {
                    return candidate
                        .canonicalize()
                        .unwrap_or_else(|_| candidate.clone());
                }
            }
        }
    }

    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../netrail/static");
    if dev.join("index.html").is_file() {
        return dev;
    }

    tracing::error!(
        "NetRail UI assets not found. Set NETRAIL_STATIC_DIR to the directory containing index.html."
    );
    dev
}

#[cfg(test)]
mod static_dir_tests {
    use super::*;

    #[test]
    fn dev_static_dir_contains_index_html() {
        let dir = resolve_static_dir();
        assert!(
            dir.join("index.html").is_file(),
            "expected index.html under {}",
            dir.display()
        );
    }

    #[test]
    fn netrail_static_dir_override() {
        let dir = static_dir();
        std::env::set_var("NETRAIL_STATIC_DIR", dir.as_os_str());
        let resolved = resolve_static_dir();
        assert!(resolved.join("index.html").is_file());
        std::env::remove_var("NETRAIL_STATIC_DIR");
    }
}

#[cfg(test)]
mod save_settings_tests {
    use super::*;
    use serial_test::serial;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn save_settings_roundtrip_uses_unique_temp_and_final_file() {
        let dir = TempDir::new().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let settings = Settings {
            max_results: 17,
            ..Settings::default()
        };
        let saved = save_settings(&settings).expect("save");
        assert_eq!(saved.max_results, 17);
        assert!(config_file().is_file());
        assert_eq!(load_settings().max_results, 17);
        // No leftover temps after a clean save.
        let leftovers: Vec<_> = config_dir()
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name != "settings.json" && name.contains("settings.json") && name.ends_with(".tmp")
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "unexpected temp leftovers: {leftovers:?}"
        );
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn corrupt_settings_falls_back_with_warning() {
        use std::sync::{Arc, Mutex};

        #[derive(Clone, Default)]
        struct Captured(Arc<Mutex<Vec<u8>>>);

        impl std::io::Write for Captured {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let dir = TempDir::new().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        std::fs::create_dir_all(config_dir()).unwrap();
        std::fs::write(config_file(), "{corrupt json").unwrap();

        let captured = Captured::default();
        let out = captured.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || out.clone())
            .with_max_level(tracing::Level::TRACE)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let settings = load_settings();
        assert_eq!(settings.max_results, Settings::default().max_results);
        let text = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        assert!(
            text.contains("corrupt"),
            "expected a corruption warning, got: {text}"
        );

        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn save_settings_concurrent_does_not_race() {
        let dir = TempDir::new().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let barrier = Arc::new(Barrier::new(16));
        let mut handles = Vec::new();
        for i in 0..16 {
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let settings = Settings {
                    max_results: 1 + (i % 50) as u32,
                    ..Settings::default()
                };
                save_settings(&settings).map(|s| s.max_results)
            }));
        }
        let mut errors = 0u32;
        for h in handles {
            match h.join().unwrap() {
                Ok(_) => {}
                Err(_) => errors += 1,
            }
        }
        assert_eq!(errors, 0, "concurrent save_settings must not fail");
        let final_settings = load_settings();
        assert!((1..=50).contains(&final_settings.max_results));
        assert!(config_file().is_file());
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn max_results_out_of_range_returns_config_max_results() {
        let settings = Settings {
            max_results: 0,
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.error_code(), "CONFIG_MAX_RESULTS");
    }

    #[test]
    fn history_ttl_over_limit_returns_config_history_ttl() {
        let settings = Settings {
            history_ttl_days: 4000,
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.error_code(), "CONFIG_HISTORY_TTL");
    }

    #[test]
    fn invalid_search_strategy_returns_config_search_strategy() {
        let settings = Settings {
            search_strategy: "parallel".into(),
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.error_code(), "CONFIG_SEARCH_STRATEGY");
    }

    #[test]
    fn metadata_searxng_url_returns_backend_error() {
        let settings = Settings {
            searxng_url: Some("http://169.254.169.254/".into()),
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.error_code(), "BACKEND_URL_CLOUD_METADATA");
    }
}