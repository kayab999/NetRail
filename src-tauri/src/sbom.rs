//! Embedded software-bill-of-materials inventory (AUDIT_ARCH §5 / E2).
//!
//! `build.rs` derives the inventory from `Cargo.lock` at build time and
//! writes it to `$OUT_DIR`; this module embeds it into every NetRail binary
//! so the shipped artifact is self-describing. `netrail-api --sbom` prints it;
//! the desktop bundles also carry the full `SBOM.txt` at
//! `/usr/share/netrail/SBOM.txt`.

pub const SBOM_INVENTORY: &str = include_str!(concat!(env!("OUT_DIR"), "/sbom_inventory.txt"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VERSION;

    #[test]
    fn embedded_inventory_is_non_empty() {
        assert!(!SBOM_INVENTORY.trim().is_empty());
    }

    #[test]
    fn embedded_inventory_has_header_and_deps() {
        assert!(SBOM_INVENTORY.starts_with("# NetRail Rust dependency inventory"));
        // Core web-server dependencies the product actually links.
        for dep in ["tokio@", "axum@", "rusqlite@", "reqwest@"] {
            assert!(SBOM_INVENTORY.lines().any(|l| l.starts_with(dep)),
                "missing {dep} in embedded inventory");
        }
        // The root package must pin the current release version.
        assert!(
            SBOM_INVENTORY.lines().any(|l| l == format!("netrail@{VERSION}")),
            "root package netrail@{VERSION} missing from inventory"
        );
    }
}
