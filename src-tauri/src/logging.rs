//! Process-wide log initialization (A5).
//!
//! Default is human-readable formatting to stderr. Set `NETRAIL_LOG_JSON=1`
//! to emit one JSON object per event (NDJSON) — ingestible by SIEM tools and
//! `jq`. Filtering is controlled by `RUST_LOG` as before.

use tracing_subscriber::EnvFilter;

pub fn init(default_filter: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| default_filter.into());
    let json = std::env::var("NETRAIL_LOG_JSON")
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(false);
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if json {
        builder.json().init();
    } else {
        builder.init();
    }
}
