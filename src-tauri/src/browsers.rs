use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::{is_flatpak, Settings};

#[derive(Debug, Clone, Serialize)]
pub struct BrowserInfo {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub supports_private: bool,
}

#[derive(Debug, Clone)]
struct BrowserSpec {
    name: &'static str,
    private_flag: Option<&'static str>,
}

fn known_browsers() -> HashMap<&'static str, BrowserSpec> {
    HashMap::from([
        ("firefox", BrowserSpec { name: "Firefox", private_flag: Some("--private-window") }),
        ("google-chrome", BrowserSpec { name: "Google Chrome", private_flag: Some("--incognito") }),
        ("chromium", BrowserSpec { name: "Chromium", private_flag: Some("--incognito") }),
        ("brave-browser", BrowserSpec { name: "Brave", private_flag: Some("--incognito") }),
        ("microsoft-edge", BrowserSpec { name: "Microsoft Edge", private_flag: Some("--inprivate") }),
        ("vivaldi", BrowserSpec { name: "Vivaldi", private_flag: Some("--incognito") }),
        ("librewolf", BrowserSpec { name: "LibreWolf", private_flag: Some("--private-window") }),
    ])
}

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }
    dirs
}

fn host_which(token: &str) -> Option<String> {
    let output = Command::new("flatpak-spawn")
        .args(["--host", "which", token])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

fn resolve_executable(command: &str) -> Option<String> {
    let token = command.split_whitespace().next()?;
    if is_flatpak() {
        if token.starts_with('/') {
            return Some(token.to_string());
        }
        return host_which(Path::new(token).file_name()?.to_str()?).or_else(|| Some(token.to_string()));
    }
    which::which(token).ok().map(|p| p.display().to_string()).or(None)
}

fn parse_desktop(path: &Path) -> Option<(String, String, bool)> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut name = path.file_stem()?.to_str()?.to_string();
    let mut exec = String::new();
    let mut is_browser = false;

    for line in content.lines() {
        if line.starts_with("Name=") {
            name = line.trim_start_matches("Name=").to_string();
        } else if line.starts_with("Exec=") {
            exec = line.trim_start_matches("Exec=").split('%').next()?.trim().to_string();
        } else if line.starts_with("Categories=") || line.starts_with("MimeType=") {
            let lower = line.to_lowercase();
            if lower.contains("webbrowser") || lower.contains("x-scheme-handler/http") {
                is_browser = true;
            }
        } else if line.starts_with("Type=") && !line.contains("Application") {
            return None;
        } else if line.starts_with("NoDisplay=true") {
            return None;
        }
    }

    if exec.is_empty() || !is_browser {
        return None;
    }
    Some((name, exec, true))
}

pub fn discover_browsers() -> Vec<BrowserInfo> {
    let known = known_browsers();
    let mut seen = Vec::new();
    let mut browsers = Vec::new();

    for dir in desktop_dirs() {
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Some((name, command, _)) = parse_desktop(&path) else { continue };
            let Some(resolved) = resolve_executable(&command) else { continue };
            if seen.contains(&resolved) {
                continue;
            }
            seen.push(resolved.clone());
            let stem = Path::new(&resolved)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("browser")
                .to_string();
            let spec = known.get(stem.as_str());
            browsers.push(BrowserInfo {
                id: stem.clone(),
                name: spec.map(|s| s.name.to_string()).unwrap_or(name),
                executable: resolved,
                supports_private: spec.and_then(|s| s.private_flag).is_some(),
            });
        }
    }

    for (stem, spec) in &known {
        let resolved = if is_flatpak() {
            host_which(stem)
        } else {
            which::which(stem).ok().map(|p| p.display().to_string())
        };
        if let Some(path) = resolved {
            if seen.contains(&path) {
                continue;
            }
            seen.push(path.clone());
            browsers.push(BrowserInfo {
                id: stem.to_string(),
                name: spec.name.to_string(),
                executable: path,
                supports_private: spec.private_flag.is_some(),
            });
        }
    }

    browsers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    browsers
}

fn find_browser(browser_id: Option<&str>) -> Option<BrowserInfo> {
    let browsers = discover_browsers();
    if browsers.is_empty() {
        return None;
    }
    if let Some(id) = browser_id {
        if let Some(found) = browsers.iter().find(|b| b.id == id) {
            return Some(found.clone());
        }
    }
    browsers.first().cloned()
}

fn private_flag_for(browser_id: &str) -> Option<&'static str> {
    known_browsers().get(browser_id).and_then(|s| s.private_flag)
}

fn spawn(mut cmd: Command) {
    cmd.env_remove("LD_PRELOAD");
    if is_flatpak() {
        let program = cmd.get_program().to_string_lossy().to_string();
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let mut wrapped = Command::new("flatpak-spawn");
        wrapped.arg("--host").arg(program);
        for arg in args {
            wrapped.arg(arg);
        }
        cmd = wrapped;
    }
    let _ = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[derive(Debug, Serialize)]
pub struct OpenResult {
    pub browser: String,
    pub executable: String,
    pub mode: String,
    pub url: String,
    pub sandbox: String,
}

pub fn open_url(url: &str, settings: &Settings, result_id: Option<i64>) -> Result<OpenResult, String> {
    let browser = find_browser(settings.browser_id.as_deref())
        .ok_or_else(|| "No web browser found on this system.".to_string())?;

    let private = settings.private_mode;
    let mut cmd = Command::new(&browser.executable);
    if private {
        if let Some(flag) = private_flag_for(&browser.id) {
            cmd.arg(flag);
        }
    }
    cmd.arg(url);
    spawn(cmd);

    let mode = if private && browser.supports_private {
        "private"
    } else {
        "normal"
    };

    if settings.history_enabled {
        if let Ok(store) = crate::history::HistoryStore::open(settings) {
            let _ = store.record_visit(url, result_id, settings.browser_id.as_deref(), private);
        }
    }

    Ok(OpenResult {
        browser: browser.name,
        executable: browser.executable,
        mode: mode.into(),
        url: url.into(),
        sandbox: if is_flatpak() { "flatpak-host" } else { "native" }.into(),
    })
}