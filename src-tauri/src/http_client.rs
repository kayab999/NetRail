use reqwest::Client;
use std::time::Duration;

pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Shared reqwest client for connection pooling and DNS caching across fanout backends.
/// Redirects are disabled so a backend cannot bounce the process onto private/metadata targets.
pub fn build_http_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .pool_max_idle_per_host(20)
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| Client::new())
}