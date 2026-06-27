use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const VERSION: &str = "0.5.0";
pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 7421;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub browser_id: Option<String>,
    pub private_mode: bool,
    pub max_results: u32,
    pub backend_order: Vec<String>,
    pub ddgs_enabled: bool,
    pub searxng_url: Option<String>,
    pub history_enabled: bool,
    pub history_encrypt: bool,
    pub history_ttl_days: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            browser_id: None,
            private_mode: false,
            max_results: 25,
            backend_order: vec!["searxng".into(), "ddgs".into()],
            ddgs_enabled: true,
            searxng_url: None,
            history_enabled: true,
            history_encrypt: true,
            history_ttl_days: 90,
        }
    }
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
    let mut settings = config_file()
        .exists()
        .then(|| fs::read_to_string(config_file()).ok())
        .flatten()
        .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
        .unwrap_or_default();

    apply_env_overrides(&mut settings);
    settings
}

pub fn save_settings(settings: &Settings) -> Settings {
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    let payload = serde_json::to_string_pretty(settings).unwrap_or_default();
    let _ = fs::write(config_file(), format!("{payload}\n"));
    load_settings()
}

fn apply_env_overrides(settings: &mut Settings) {
    if let Ok(url) = env::var("NETRAIL_SEARXNG_URL").or_else(|_| env::var("SEARXNG_URL")) {
        if !url.is_empty() {
            settings.searxng_url = Some(url);
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

pub fn static_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../netrail/static")
}