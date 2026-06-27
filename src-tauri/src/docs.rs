use std::path::PathBuf;

const MANUAL_MD: &str = include_str!("../../docs/MANUAL.md");
const ABOUT_MD: &str = include_str!("../../README.md");

pub fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

pub fn load_doc(slug: &str) -> Result<serde_json::Value, String> {
    let (title, markdown) = match slug {
        "manual" => ("User Manual", MANUAL_MD),
        "about" => ("About NetRail", ABOUT_MD),
        _ => return Err("Unknown document.".into()),
    };
    let markdown = rewrite_asset_paths(markdown);
    Ok(serde_json::json!({
        "slug": slug,
        "title": title,
        "markdown": markdown,
    }))
}

pub fn asset_path(filename: &str) -> Option<PathBuf> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return None;
    }
    let static_asset = crate::config::static_dir().join("docs/assets").join(filename);
    if static_asset.exists() {
        return Some(static_asset);
    }
    let project_asset = project_root().join("docs/assets").join(filename);
    if project_asset.exists() {
        return Some(project_asset);
    }
    None
}

fn rewrite_asset_paths(markdown: &str) -> String {
    markdown
        .replace("](docs/assets/", "](/static/docs/assets/")
        .replace("](docs/assets\\", "](/static/docs/assets/")
}