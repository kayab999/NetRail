// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--api-only") {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "netrail=info".into()),
            )
            .init();
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