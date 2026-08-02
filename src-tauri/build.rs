use std::path::Path;

fn main() {
    #[cfg(feature = "desktop")]
    tauri_build::build();

    generate_sbom_inventory();
}

/// Writes `$OUT_DIR/sbom_inventory.txt`: one `name@version` per Cargo.lock
/// package (sorted, deduped). The text is embedded into every binary via
/// `include_str!` in `src/sbom.rs` and printed by `netrail-api --sbom`, so a
/// shipped artifact is self-describing (AUDIT_ARCH §5 / E2). Deterministic:
/// no timestamps, so identical inputs produce identical output.
fn generate_sbom_inventory() {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let lock_path = Path::new(&manifest_dir).join("Cargo.lock");
    let lock_text = std::fs::read_to_string(&lock_path).expect("read Cargo.lock");

    let mut entries: Vec<String> = Vec::new();
    let mut name: Option<String> = None;
    for line in lock_text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name =") {
            name = Some(rest.trim().trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("version =") {
            if let Some(n) = name.take() {
                entries.push(format!("{n}@{}", rest.trim().trim_matches('"')));
            }
        }
    }
    entries.sort();
    entries.dedup();

    let mut content = String::from("# NetRail Rust dependency inventory (embedded at build time)\n");
    for entry in entries {
        content.push_str(&entry);
        content.push('\n');
    }

    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR");
    std::fs::write(Path::new(&out_dir).join("sbom_inventory.txt"), content)
        .expect("write sbom_inventory.txt");

    println!("cargo:rerun-if-changed=Cargo.lock");
}
