use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::{is_flatpak, Settings};
use crate::error::{NetRailError, NetRailResult};

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

// Canonical known-browser table (QA-09 T2) — single source of truth is
// tests/fixtures/browsers.json; tests assert this table equals the fixture.
// Keep order deterministic (binary name ascending); HashMap iteration would
// make fallback discovery order nondeterministic.
fn known_browsers() -> Vec<(&'static str, BrowserSpec)> {
    vec![
        ("firefox", BrowserSpec { name: "Firefox", private_flag: Some("--private-window") }),
        ("firefox-esr", BrowserSpec { name: "Firefox ESR", private_flag: Some("--private-window") }),
        ("google-chrome", BrowserSpec { name: "Google Chrome", private_flag: Some("--incognito") }),
        ("google-chrome-stable", BrowserSpec { name: "Google Chrome", private_flag: Some("--incognito") }),
        ("chromium", BrowserSpec { name: "Chromium", private_flag: Some("--incognito") }),
        ("chromium-browser", BrowserSpec { name: "Chromium", private_flag: Some("--incognito") }),
        ("brave-browser", BrowserSpec { name: "Brave", private_flag: Some("--incognito") }),
        ("microsoft-edge", BrowserSpec { name: "Microsoft Edge", private_flag: Some("--inprivate") }),
        ("microsoft-edge-stable", BrowserSpec { name: "Microsoft Edge", private_flag: Some("--inprivate") }),
        ("opera", BrowserSpec { name: "Opera", private_flag: Some("--private") }),
        ("vivaldi", BrowserSpec { name: "Vivaldi", private_flag: Some("--incognito") }),
        ("waterfox", BrowserSpec { name: "Waterfox", private_flag: Some("--private-window") }),
        ("librewolf", BrowserSpec { name: "LibreWolf", private_flag: Some("--private-window") }),
    ]
}

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }
    dirs
}

fn host_which(token: &str) -> Option<String> {
    let basename = Path::new(token).file_name()?.to_str()?.to_string();
    let output = Command::new("flatpak-spawn")
        .args(["--host", "which", &basename])
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
    // Desktop entries are section-scoped (QA-09 T2): keys of [Desktop Action …]
    // and localized Name[xx]= variants must NOT leak into the entry (a later
    // action section used to override the entry Name).
    let mut in_entry = false;
    let mut saw_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_entry = line == "[Desktop Entry]";
            saw_entry |= in_entry;
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some(rest) = line.strip_prefix("Name=") {
            name = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("Exec=") {
            exec = rest.split('%').next()?.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Categories=") {
            if rest.to_lowercase().contains("webbrowser") {
                is_browser = true;
            }
        } else if let Some(rest) = line.strip_prefix("MimeType=") {
            if rest.to_lowercase().contains("x-scheme-handler/http") {
                is_browser = true;
            }
        } else if let Some(rest) = line.strip_prefix("Type=") {
            if rest.trim() != "Application" {
                return None;
            }
        } else if let Some(rest) = line.strip_prefix("NoDisplay=") {
            if rest.trim().eq_ignore_ascii_case("true") {
                return None;
            }
        }
    }

    if !saw_entry || exec.is_empty() || !is_browser {
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
            let stem_name = Path::new(&resolved)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("browser")
                .to_string();
            let spec = known.iter().find(|(stem, _)| *stem == stem_name.as_str());
            browsers.push(BrowserInfo {
                id: stem_name.clone(),
                name: spec.map(|s| s.1.name.to_string()).unwrap_or(name),
                executable: resolved,
                supports_private: spec.and_then(|s| s.1.private_flag).is_some(),
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

    browsers.sort_by_key(|b| b.name.to_lowercase());
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
    known_browsers()
        .iter()
        .find(|(stem, _)| *stem == browser_id)
        .and_then(|(_, spec)| spec.private_flag)
}

fn spawn(mut cmd: Command) -> Result<(), std::io::Error> {
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
    // A-16: a failed launch (ENOENT, fork failure) must surface as an error,
    // not a silent success response. Stdio::null has no error path; only the
    // spawn result is propagated.
    cmd.stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct OpenResult {
    pub browser: String,
    pub executable: String,
    pub mode: String,
    pub url: String,
    pub sandbox: String,
}

fn dry_run_enabled() -> bool {
    std::env::var("NETRAIL_NO_OPEN")
        .map(|v| !v.trim().is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(false)
}

pub fn open_url(url: &str, settings: &Settings) -> NetRailResult<OpenResult> {
    // NETRAIL_NO_OPEN: harness/dry-run mode — report success without
    // discovering or spawning a browser (used by the parity smoke script).
    if dry_run_enabled() {
        return Ok(OpenResult {
            browser: "dry-run".into(),
            executable: "dry-run".into(),
            mode: if settings.private_mode { "private" } else { "normal" }.into(),
            url: url.into(),
            sandbox: "dry-run".into(),
        });
    }

    let browser = find_browser(settings.browser_id.as_deref()).ok_or_else(|| {
        NetRailError::Internal {
            code: "BROWSER_NOT_FOUND",
            message: "No web browser found on this system.".into(),
        }
    })?;

    let private = settings.private_mode;
    let mut cmd = Command::new(&browser.executable);
    if private {
        if let Some(flag) = private_flag_for(&browser.id) {
            cmd.arg(flag);
        }
    }
    cmd.arg(url);
    spawn(cmd).map_err(|err| NetRailError::Internal {
        code: "BROWSER_SPAWN_FAILED",
        message: format!(
            "Failed to launch browser {} ({}): {}",
            browser.executable,
            browser.name,
            err
        ),
    })?;

    let mode = if private && browser.supports_private {
        "private"
    } else {
        "normal"
    };

    Ok(OpenResult {
        browser: browser.name,
        executable: browser.executable,
        mode: mode.into(),
        url: url.into(),
        sandbox: if is_flatpak() { "flatpak-host" } else { "native" }.into(),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use super::*;
    use crate::config::Settings;
    use serde::Deserialize;
    use serial_test::serial;
    #[derive(Deserialize)]
    struct Fixture {
        known_browsers: Vec<FixtureBrowser>,
    }

    #[derive(Deserialize)]
    struct FixtureBrowser {
        id: String,
        name: String,
        private_flag: Option<String>,
    }

    fn write_desktop(lines: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("test-browser.desktop");
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(lines.join("\n").as_bytes()).expect("write");
        drop(f);
        dir
    }

    fn browser_desktop_lines<'a>(extra: &'a [&'a str]) -> Vec<&'a str> {
        let mut lines: Vec<&'a str> = vec![
            "[Desktop Entry]",
            "Type=Application",
            "Name=Test Browser",
            "Exec=test-browser %U",
            "MimeType=x-scheme-handler/http;",
        ];
        lines.extend_from_slice(extra);
        lines
    }

    #[test]
    fn known_table_matches_fixture() {
        let fixture: Fixture =
            serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/browsers.json"))).expect("fixture parses");
        let known = known_browsers();
        assert_eq!(known.len(), fixture.known_browsers.len(),
                   "known table must match fixture (drift detected)");
        for (stem, spec) in &known {
            let want = fixture.known_browsers.iter()
                .find(|b| &b.id == stem)
                .unwrap_or_else(|| panic!("fixture missing {stem}"));
            assert_eq!(spec.name, want.name, "name mismatch for {stem}");
            assert_eq!(spec.private_flag.map(str::to_string),
                       want.private_flag, "flag mismatch for {stem}");
        }
    }

    #[test]
    fn parse_desktop_scopes_name_to_entry_section() {
        let dir = write_desktop(&[
            "[Desktop Entry]",
            "Type=Application",
            "Name=Test Browser",
            "Exec=test-browser %U",
            "MimeType=x-scheme-handler/http;",
            "",
            "[Desktop Action new-window]",
            "Name=New Window",
        ]);
        let (name, exec, _) =
            parse_desktop(&dir.path().join("test-browser.desktop")).expect("parses");
        assert_eq!(name, "Test Browser", "action section Name leaked into entry");
        assert_eq!(exec, "test-browser");
    }

    #[test]
    fn parse_desktop_ignores_localized_name_variant() {
        let dir = write_desktop(&browser_desktop_lines(&[
            "Name[de]=Lokaler Browser",
        ]));
        let (name, _, _) =
            parse_desktop(&dir.path().join("test-browser.desktop")).expect("parses");
        assert_eq!(name, "Test Browser", "localized Name[xx]= must not override Name");
    }

    #[test]
    fn parse_desktop_requires_exact_application_type() {
        for bad in ["Application2", "X-Application", "application"] {
            let dir = write_desktop(&browser_desktop_lines(&[
                &format!("Type={bad}"),
            ]));
            assert!(parse_desktop(&dir.path().join("test-browser.desktop")).is_none(),
                    "Type={bad} must be rejected");
        }
    }

    #[test]
    fn parse_desktop_nodisplay_only_rejected_inside_entry() {
        let inside = write_desktop(&browser_desktop_lines(&["NoDisplay=true"]));
        assert!(parse_desktop(&inside.path().join("test-browser.desktop")).is_none());
        let outside = write_desktop(&browser_desktop_lines(&[
            "",
            "[Desktop Action foo]",
            "NoDisplay=true",
        ]));
        assert!(parse_desktop(&outside.path().join("test-browser.desktop")).is_some(),
                "NoDisplay outside [Desktop Entry] must be ignored");
    }

    #[test]
    fn parse_desktop_requires_entry_section_header() {
        let dir = write_desktop(&[
            "Type=Application",
            "Name=No Section",
            "Exec=test-browser",
            "Categories=Network;",
        ]);
        assert!(parse_desktop(&dir.path().join("test-browser.desktop")).is_none(),
                "missing [Desktop Entry] header must be rejected (configparser parity)");
    }

    #[test]
    fn parse_desktop_truncates_exec_at_percent() {
        let dir = write_desktop(&browser_desktop_lines(&[
            "Exec=env FOO=1 test-browser --profile %u %U",
        ]));
        let (_, exec, _) =
            parse_desktop(&dir.path().join("test-browser.desktop")).expect("parses");
        assert_eq!(exec, "env FOO=1 test-browser --profile");
    }

    #[test]
    #[serial]
    fn unknown_browser_gets_no_fabricated_private_flag() {
        let home = tempfile::TempDir::new().expect("tempdir");
        let bin = tempfile::TempDir::new().expect("tempdir");
        let bin_path = bin.path().join("unknown-browser-bin");
        std::fs::write(&bin_path, "#!/bin/sh\n").expect("fake binary");
        let mut perms = std::fs::metadata(&bin_path).expect("meta").permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin_path, perms).expect("chmod");

        let apps = home.path().join(".local/share/applications");
        std::fs::create_dir_all(&apps).expect("apps dir");
        let mut f = std::fs::File::create(apps.join("unknown-browser.desktop")).expect("create");
        writeln!(f, "[Desktop Entry]\nType=Application\nName=Unknown Co.\nExec={}\nCategories=Network;WebBrowser;",
                 bin_path.display()).expect("write");

        std::env::set_var("HOME", home.path());
        let mut path = bin.path().as_os_str().to_owned();
        path.push(":");
        path.push(std::env::var_os("PATH").unwrap_or_default().as_os_str());
        std::env::set_var("PATH", &path);

        let found = discover_browsers();
        std::env::remove_var("HOME");
        std::env::remove_var("PATH");

        let unknown = found.iter().find(|b| b.id == "unknown-browser-bin")
            .expect("fake browser discovered");
        assert_eq!(unknown.name, "Unknown Co.", "display name must come from the entry");
        assert!(!unknown.supports_private,
                "unknown binaries must not be claimed private-capable");
        assert!(private_flag_for(&unknown.id).is_none(),
                "unknown binaries must never receive a fabricated private flag");
    }

    #[test]
    #[serial]
    fn netrail_no_open_returns_dry_run_without_discovery() {
        std::env::set_var("NETRAIL_NO_OPEN", "1");
        let settings = Settings {
            browser_id: Some("definitely-not-installed".into()),
            private_mode: true,
            ..Settings::default()
        };
        let result = open_url("https://example.com/", &settings).expect("dry-run must succeed");
        assert_eq!(result.browser, "dry-run");
        assert_eq!(result.executable, "dry-run");
        assert_eq!(result.mode, "private");
        assert_eq!(result.url, "https://example.com/");
        std::env::remove_var("NETRAIL_NO_OPEN");
    }

    #[test]
    #[serial]
    fn dry_run_env_parsing() {
        for (value, expected) in [
            ("1", true),
            ("true", true),
            ("yes", true),
            ("0", false),
            ("false", false),
            ("FALSE", false),
            ("", false),
        ] {
            std::env::set_var("NETRAIL_NO_OPEN", value);
            assert_eq!(dry_run_enabled(), expected, "NETRAIL_NO_OPEN={value}");
        }
        std::env::remove_var("NETRAIL_NO_OPEN");
        assert!(!dry_run_enabled());
    }

    #[test]
    fn spawn_reports_failure_for_missing_executable() {
        let mut cmd = std::process::Command::new("/definitely/not/a/real/browser");
        cmd.arg("https://example.com/");
        assert!(
            spawn(cmd).is_err(),
            "A-16: a failed launch must surface as Err, not silent success"
        );
    }
}