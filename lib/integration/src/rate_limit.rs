//! Rate limiting for integration operations.
//!
//! Respects external API constraints by limiting request rates.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window.
    pub max_requests: u32,
    /// Window duration in seconds.
    pub window_seconds: u32,
    /// Optional retry-after behavior.
    pub retry_behavior: RetryBehavior,
}

impl RateLimitConfig {
    /// Creates a new rate limit configuration.
    #[must_use]
    pub fn new(max_requests: u32, window_seconds: u32) -> Self {
        Self {
            max_requests,
            window_seconds,
            retry_behavior: RetryBehavior::default(),
        }
    }

    /// Common limit: 100 requests per minute.
    #[must_use]
    pub fn per_minute(max_requests: u32) -> Self {
        Self::new(max_requests, 60)
    }

    /// Common limit: requests per hour.
    #[must_use]
    pub fn per_hour(max_requests: u32) -> Self {
        Self::new(max_requests, 3600)
    }

    /// Common limit: requests per day.
    #[must_use]
    pub fn per_day(max_requests: u32) -> Self {
        Self::new(max_requests, 86400)
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::per_minute(60)
    }
}

/// Behavior when rate limit is exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryBehavior {
    /// Fail immediately.
    #[default]
    FailFast,
    /// Wait and retry.
    WaitAndRetry,
    /// Queue for later execution.
    Queue,
}

/// Result of a rate limit check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request is allowed.
    Allowed {
        remaining: u32,
        resets_at: DateTime<Utc>,
    },
    /// Rate limit exceeded.
    Exceeded {
        retry_after: Duration,
        resets_at: DateTime<Utc>,
    },
}

impl RateLimitResult {
    /// Returns true if the request is allowed.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. })
    }

    /// Returns the number of remaining requests (0 if exceeded).
    #[must_use]
    pub fn remaining(&self) -> u32 {
        match self {
            Self::Allowed { remaining, .. } => *remaining,
            Self::Exceeded { .. } => 0,
        }
    }
}

/// State for a single rate limit window.
#[derive(Debug, Clone)]
struct WindowState {
    /// Number of requests made in this window.
    count: u32,
    /// When this window started.
    window_start: DateTime<Utc>,
}

impl WindowState {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Utc::now(),
        }
    }
}

/// A rate limiter for integration requests.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    /// State per integration account ID.
    state: Arc<RwLock<HashMap<String, WindowState>>>,
}

impl RateLimiter {
    /// Creates a new rate limiter with the given configuration.
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Checks if a request is allowed for the given key.
    ///
    /// If allowed, increments the request count.
    pub fn check_and_increment(&self, key: &str) -> RateLimitResult {
        let mut state = self.state.write().unwrap();
        let now = Utc::now();
        let window_duration = Duration::seconds(self.config.window_seconds as i64);

        let window_state = state.entry(key.to_string()).or_insert_with(WindowState::new);

        // Check if we need to start a new window
        if now - window_state.window_start >= window_duration {
            window_state.window_start = now;
            window_state.count = 0;
        }

        let resets_at = window_state.window_start + window_duration;

        if window_state.count >= self.config.max_requests {
            let retry_after = resets_at - now;
            return RateLimitResult::Exceeded {
                retry_after,
                resets_at,
            };
        }

        window_state.count += 1;
        let remaining = self.config.max_requests - window_state.count;

        RateLimitResult::Allowed {
            remaining,
            resets_at,
        }
    }

    /// Checks if a request would be allowed without incrementing.
    #[must_use]
    pub fn check(&self, key: &str) -> RateLimitResult {
        let state = self.state.read().unwrap();
        let now = Utc::now();
        let window_duration = Duration::seconds(self.config.window_seconds as i64);

        let Some(window_state) = state.get(key) else {
            // No state means no requests yet
            return RateLimitResult::Allowed {
                remaining: self.config.max_requests,
                resets_at: now + window_duration,
            };
        };

        // Check if window has expired
        if now - window_state.window_start >= window_duration {
            return RateLimitResult::Allowed {
                remaining: self.config.max_requests,
                resets_at: now + window_duration,
            };
        }

        let resets_at = window_state.window_start + window_duration;

        if window_state.count >= self.config.max_requests {
            let retry_after = resets_at - now;
            return RateLimitResult::Exceeded {
                retry_after,
                resets_at,
            };
        }

        let remaining = self.config.max_requests - window_state.count;
        RateLimitResult::Allowed {
            remaining,
            resets_at,
        }
    }

    /// Resets the rate limit for a key.
    pub fn reset(&self, key: &str) {
        let mut state = self.state.write().unwrap();
        state.remove(key);
    }

    /// Returns the current configuration.
    #[must_use]
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_allows_under_limit() {
        let limiter = RateLimiter::new(RateLimitConfig::new(10, 60));

        for i in 0..10 {
            let result = limiter.check_and_increment("test");
            assert!(result.is_allowed());
            assert_eq!(result.remaining(), 10 - i - 1);
        }
    }

    #[test]
    fn rate_limit_blocks_over_limit() {
        let limiter = RateLimiter::new(RateLimitConfig::new(5, 60));

        // Use up the limit
        for _ in 0..5 {
            let result = limiter.check_and_increment("test");
            assert!(result.is_allowed());
        }

        // Next request should be blocked
        let result = limiter.check_and_increment("test");
        assert!(!result.is_allowed());
        assert_eq!(result.remaining(), 0);
    }

    #[test]
    fn rate_limit_per_key_isolation() {
        let limiter = RateLimiter::new(RateLimitConfig::new(2, 60));

        // Use up limit for key1
        limiter.check_and_increment("key1");
        limiter.check_and_increment("key1");

        // key1 should be blocked
        assert!(!limiter.check("key1").is_allowed());

        // key2 should still be allowed
        assert!(limiter.check("key2").is_allowed());
    }

    #[test]
    fn rate_limit_reset() {
        let limiter = RateLimiter::new(RateLimitConfig::new(2, 60));

        limiter.check_and_increment("test");
        limiter.check_and_increment("test");
        assert!(!limiter.check("test").is_allowed());

        limiter.reset("test");
        assert!(limiter.check("test").is_allowed());
    }

    #[test]
    fn rate_limit_config_presets() {
        let per_minute = RateLimitConfig::per_minute(100);
        assert_eq!(per_minute.max_requests, 100);
        assert_eq!(per_minute.window_seconds, 60);

        let per_hour = RateLimitConfig::per_hour(1000);
        assert_eq!(per_hour.max_requests, 1000);
        assert_eq!(per_hour.window_seconds, 3600);

        let per_day = RateLimitConfig::per_day(10000);
        assert_eq!(per_day.max_requests, 10000);
        assert_eq!(per_day.window_seconds, 86400);
    }
}
