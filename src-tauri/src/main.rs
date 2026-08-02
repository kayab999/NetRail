// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--sbom") {
        print!("{}", netrail_lib::sbom::SBOM_INVENTORY);
        return;
    }
    if std::env::args().any(|arg| arg == "--api-only") {
        netrail_lib::logging::init("netrail=info");
        tauri::async_runtime::block_on(async {
            if let Err(err) = netrail_lib::server::start().await {
                eprintln!("NetRail API server failed: {err}");
                std::process::exit(1);
            }
        });
        return;
    }
    netrail_lib::run();
}