//! Lightweight localhost rate limits for search/open abuse protection.
//! Disable with `NETRAIL_RATE_LIMIT=0`.
//!
//! Buckets are keyed by client identity: when token auth is on, each token
//! gets its own per-minute budget (`client_identity` in auth.rs); otherwise
//! everything shares one process-wide budget. Limits are per-process — two
//! API processes (desktop + Docker) do not share counters (A9).

use crate::error::{NetRailError, NetRailResult};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_SEARCH_PER_MIN: u32 = 90;
const DEFAULT_OPEN_PER_MIN: u32 = 120;
/// Settings / history purge / collections mutations.
const DEFAULT_MUTATE_PER_MIN: u32 = 60;
const WINDOW: Duration = Duration::from_secs(60);
/// Drop buckets idle for two windows when the identity set grows past this.
const MAX_IDENTITIES: usize = 1024;

#[derive(Debug)]
struct WindowCounter {
    window_start: Instant,
    count: u32,
}

impl WindowCounter {
    fn try_acquire(&mut self, limit: u32) -> bool {
        if limit == 0 {
            return true;
        }
        let now = Instant::now();
        if now.duration_since(self.window_start) >= WINDOW {
            self.window_start = now;
            self.count = 0;
        }
        if self.count >= limit {
            return false;
        }
        self.count += 1;
        true
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    search: Arc<Mutex<HashMap<String, WindowCounter>>>,
    open: Arc<Mutex<HashMap<String, WindowCounter>>>,
    mutate: Arc<Mutex<HashMap<String, WindowCounter>>>,
    search_limit: u32,
    open_limit: u32,
    mutate_limit: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::from_env()
    }
}

fn acquire(map: &Mutex<HashMap<String, WindowCounter>>, identity: &str, limit: u32) -> bool {
    if limit == 0 {
        return true;
    }
    let mut guard = map.lock();
    let counter = guard
        .entry(identity.to_string())
        .or_insert_with(|| WindowCounter {
            window_start: Instant::now(),
            count: 0,
        });
    let ok = counter.try_acquire(limit);
    if guard.len() > MAX_IDENTITIES {
        let now = Instant::now();
        guard.retain(|_, c| now.duration_since(c.window_start) < WINDOW * 2);
    }
    ok
}

impl RateLimiter {
    pub fn from_env() -> Self {
        let enabled = env::var("NETRAIL_RATE_LIMIT")
            .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
            .unwrap_or(true);
        Self::with_limits(
            if enabled { DEFAULT_SEARCH_PER_MIN } else { 0 },
            if enabled { DEFAULT_OPEN_PER_MIN } else { 0 },
            if enabled { DEFAULT_MUTATE_PER_MIN } else { 0 },
        )
    }

    /// Test helper: build limiter with explicit per-window caps (0 = unlimited).
    pub fn with_limits(search: u32, open: u32, mutate: u32) -> Self {
        Self {
            search: Arc::new(Mutex::new(HashMap::new())),
            open: Arc::new(Mutex::new(HashMap::new())),
            mutate: Arc::new(Mutex::new(HashMap::new())),
            search_limit: search,
            open_limit: open,
            mutate_limit: mutate,
        }
    }

    pub fn check_search(&self, identity: &str) -> NetRailResult<()> {
        if acquire(&self.search, identity, self.search_limit) {
            Ok(())
        } else {
            Err(NetRailError::RateLimited {
                code: "RATE_LIMITED",
                message: format!(
                    "Too many searches (max {DEFAULT_SEARCH_PER_MIN}/minute). Wait a moment."
                ),
            })
        }
    }

    pub fn check_open(&self, identity: &str) -> NetRailResult<()> {
        if acquire(&self.open, identity, self.open_limit) {
            Ok(())
        } else {
            Err(NetRailError::RateLimited {
                code: "RATE_LIMITED",
                message: format!(
                    "Too many open requests (max {DEFAULT_OPEN_PER_MIN}/minute). Wait a moment."
                ),
            })
        }
    }

    pub fn check_mutate(&self, identity: &str) -> NetRailResult<()> {
        if acquire(&self.mutate, identity, self.mutate_limit) {
            Ok(())
        } else {
            Err(NetRailError::RateLimited {
                code: "RATE_LIMITED",
                message: format!(
                    "Too many configuration/history mutations (max {DEFAULT_MUTATE_PER_MIN}/minute)."
                ),
            })
        }
    }

    pub fn status_json(&self) -> serde_json::Value {
        let enabled = env::var("NETRAIL_RATE_LIMIT")
            .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
            .unwrap_or(true);
        serde_json::json!({
            "enabled": enabled,
            "mode": if crate::auth::token_required() { "per-token" } else { "process" },
            "search_per_minute": DEFAULT_SEARCH_PER_MIN,
            "open_per_minute": DEFAULT_OPEN_PER_MIN,
            "mutate_per_minute": DEFAULT_MUTATE_PER_MIN,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_under_limit() {
        let mut c = WindowCounter {
            window_start: Instant::now(),
            count: 0,
        };
        assert!(c.try_acquire(3));
        assert!(c.try_acquire(3));
        assert!(c.try_acquire(3));
        assert!(!c.try_acquire(3));
    }

    #[test]
    fn zero_limit_disables() {
        let mut c = WindowCounter {
            window_start: Instant::now(),
            count: 0,
        };
        for _ in 0..20 {
            assert!(c.try_acquire(0));
        }
    }

    #[test]
    fn buckets_are_keyed_by_identity() {
        let limiter = RateLimiter::with_limits(2, 0, 0);
        assert!(limiter.check_search("alice").is_ok());
        assert!(limiter.check_search("alice").is_ok());
        assert!(limiter.check_search("alice").is_err());
        assert!(limiter.check_search("bob").is_ok());
        assert!(limiter.check_open("alice").is_ok());
    }
}
