//! Lightweight localhost rate limits for search/open abuse protection.
//! Disable with `NETRAIL_RATE_LIMIT=0`.

use crate::error::{NetRailError, NetRailResult};
use parking_lot::Mutex;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_SEARCH_PER_MIN: u32 = 90;
const DEFAULT_OPEN_PER_MIN: u32 = 120;
const WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug)]
struct WindowCounter {
    window_start: Instant,
    count: u32,
    limit: u32,
}

impl WindowCounter {
    fn new(limit: u32) -> Self {
        Self {
            window_start: Instant::now(),
            count: 0,
            limit,
        }
    }

    fn try_acquire(&mut self) -> bool {
        if self.limit == 0 {
            return true;
        }
        let now = Instant::now();
        if now.duration_since(self.window_start) >= WINDOW {
            self.window_start = now;
            self.count = 0;
        }
        if self.count >= self.limit {
            return false;
        }
        self.count += 1;
        true
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    search: Arc<Mutex<WindowCounter>>,
    open: Arc<Mutex<WindowCounter>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::from_env()
    }
}

impl RateLimiter {
    pub fn from_env() -> Self {
        let enabled = env::var("NETRAIL_RATE_LIMIT")
            .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
            .unwrap_or(true);
        let search_limit = if enabled { DEFAULT_SEARCH_PER_MIN } else { 0 };
        let open_limit = if enabled { DEFAULT_OPEN_PER_MIN } else { 0 };
        Self {
            search: Arc::new(Mutex::new(WindowCounter::new(search_limit))),
            open: Arc::new(Mutex::new(WindowCounter::new(open_limit))),
        }
    }

    pub fn check_search(&self) -> NetRailResult<()> {
        if self.search.lock().try_acquire() {
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

    pub fn check_open(&self) -> NetRailResult<()> {
        if self.open.lock().try_acquire() {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_under_limit() {
        let mut c = WindowCounter::new(3);
        assert!(c.try_acquire());
        assert!(c.try_acquire());
        assert!(c.try_acquire());
        assert!(!c.try_acquire());
    }

    #[test]
    fn zero_limit_disables() {
        let mut c = WindowCounter::new(0);
        for _ in 0..20 {
            assert!(c.try_acquire());
        }
    }
}
