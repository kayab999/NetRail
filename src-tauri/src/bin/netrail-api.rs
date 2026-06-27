//! Headless NetRail API server — no Tauri GUI, minimal footprint.
//! Build: cargo build --release --bin netrail-api --no-default-features

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "netrail=info".into()),
        )
        .init();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        if let Err(err) = netrail_lib::server::start().await {
            eprintln!("NetRail API server failed: {err}");
            std::process::exit(1);
        }
    });
}