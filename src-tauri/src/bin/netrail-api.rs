//! Headless NetRail API server — no Tauri GUI, minimal footprint.
//! Build: cargo build --release --bin netrail-api --no-default-features

fn main() {
    if std::env::args().any(|arg| arg == "--sbom") {
        print!("{}", netrail_lib::sbom::SBOM_INVENTORY);
        return;
    }

    netrail_lib::logging::init("netrail=info");

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        if let Err(err) = netrail_lib::server::start().await {
            eprintln!("NetRail API server failed: {err}");
            std::process::exit(1);
        }
    });
}
