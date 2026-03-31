//! Retry policies, backoff strategies, and rate limiting.
//!
//! Provides configurable retry logic with exponential/linear/constant backoff,
//! optional jitter, and a token bucket rate limiter for API requests.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Strategy for calculating the delay between retry attempts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Constant delay between retries.
    Constant {
        /// The fixed delay in milliseconds.
        delay_ms: u64,
    },
    /// Linearly increasing delay.
    Linear {
        /// Initial delay in milliseconds.
        initial_ms: u64,
        /// Increment per attempt in milliseconds.
        increment_ms: u64,
    },
    /// Exponentially increasing delay.
    Exponential {
        /// Initial delay in milliseconds.
        initial_ms: u64,
        /// Multiplier applied each attempt.
        multiplier: f64,
        /// Maximum delay cap in milliseconds.
        max_ms: u64,
    },
}

impl BackoffStrategy {
    /// Create a constant backoff with the given delay.
    pub fn constant(delay_ms: u64) -> Self {
        BackoffStrategy::Constant { delay_ms }
    }

    /// Create a linear backoff.
    pub fn linear(initial_ms: u64, increment_ms: u64) -> Self {
        BackoffStrategy::Linear {
            initial_ms,
            increment_ms,
        }
    }

    /// Create an exponential backoff with default settings.
    pub fn exponential() -> Self {
        BackoffStrategy::Exponential {
            initial_ms: 1000,
            multiplier: 2.0,
            max_ms: 60_000,
        }
    }

    /// Create an exponential backoff with custom parameters.
    pub fn exponential_custom(initial_ms: u64, multiplier: f64, max_ms: u64) -> Self {
        BackoffStrategy::Exponential {
            initial_ms,
            multiplier,
            max_ms,
        }
    }

    /// Calculate the delay for a given attempt number (0-based).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let ms = match self {
            BackoffStrategy::Constant { delay_ms } => *delay_ms,
            BackoffStrategy::Linear {
                initial_ms,
                increment_ms,
            } => initial_ms + increment_ms * attempt as u64,
            BackoffStrategy::Exponential {
                initial_ms,
                multiplier,
                max_ms,
            } => {
                let delay = *initial_ms as f64 * multiplier.powi(attempt as i32);
                (delay as u64).min(*max_ms)
            }
        };
        Duration::from_millis(ms)
    }
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::exponential()
    }
}

impl fmt::Display for BackoffStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackoffStrategy::Constant { delay_ms } => write!(f, "constant({delay_ms}ms)"),
            BackoffStrategy::Linear {
                initial_ms,
                increment_ms,
            } => write!(f, "linear({initial_ms}ms + {increment_ms}ms/attempt)"),
            BackoffStrategy::Exponential {
                initial_ms,
                multiplier,
                max_ms,
            } => write!(f, "exponential({initial_ms}ms * {multiplier}x, max {max_ms}ms)"),
        }
    }
}

/// Configuration for retry behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries).
    pub max_attempts: u32,
    /// The backoff strategy.
    pub backoff: BackoffStrategy,
    /// Whether to add random jitter to delays.
    pub jitter: bool,
    /// Maximum jitter as a fraction of the delay (0.0-1.0).
    pub jitter_factor: f64,
    /// HTTP status codes that should be retried.
    pub retryable_status_codes: Vec<u16>,
    /// Total timeout across all attempts.
    pub total_timeout: Option<Duration>,
}

impl RetryPolicy {
    /// Create a policy with no retries.
    pub fn none() -> Self {
        Self {
            max_attempts: 0,
            backoff: BackoffStrategy::constant(0),
            jitter: false,
            jitter_factor: 0.0,
            retryable_status_codes: Vec::new(),
            total_timeout: None,
        }
    }

    /// Create a reasonable default retry policy for API calls.
    pub fn default_api() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::exponential(),
            jitter: true,
            jitter_factor: 0.25,
            retryable_status_codes: vec![429, 500, 502, 503, 504],
            total_timeout: Some(Duration::from_secs(120)),
        }
    }

    /// Create an aggressive retry policy for critical operations.
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            backoff: BackoffStrategy::exponential_custom(500, 2.0, 30_000),
            jitter: true,
            jitter_factor: 0.3,
            retryable_status_codes: vec![429, 500, 502, 503, 504, 408],
            total_timeout: Some(Duration::from_secs(300)),
        }
    }

    /// Check if a status code is retryable under this policy.
    pub fn should_retry_status(&self, status: u16) -> bool {
        self.retryable_status_codes.contains(&status)
    }

    /// Get the delay for a specific attempt, including jitter.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = self.backoff.delay_for_attempt(attempt);
        if self.jitter {
            // Deterministic "jitter" for reproducibility in non-async context.
            let jitter_ms = (base.as_millis() as f64 * self.jitter_factor) as u64;
            // Use attempt number as a poor-man's seed for variation.
            let offset = (jitter_ms as u32).wrapping_mul(attempt.wrapping_add(1)) as u64 % (jitter_ms + 1);
            base + Duration::from_millis(offset)
        } else {
            base
        }
    }

    /// Check if another attempt is allowed.
    pub fn has_attempts_remaining(&self, current_attempt: u32) -> bool {
        current_attempt < self.max_attempts
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::default_api()
    }
}

impl fmt::Display for RetryPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "max {} attempts, {}, jitter={}",
            self.max_attempts, self.backoff, self.jitter
        )
    }
}

/// The outcome of a retry attempt.
#[derive(Debug, Clone)]
pub struct RetryOutcome<T> {
    /// The final result (success or last error).
    pub result: Result<T, RetryError>,
    /// Total number of attempts made.
    pub attempts: u32,
    /// Total time spent across all attempts.
    pub total_duration: Duration,
    /// Durations of individual attempts.
    pub attempt_durations: Vec<Duration>,
}

/// Error from a retry sequence.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RetryError {
    /// All retry attempts exhausted.
    #[error("exhausted {attempts} retry attempts: {last_error}")]
    Exhausted {
        /// Number of attempts made.
        attempts: u32,
        /// The last error message.
        last_error: String,
    },
    /// Total timeout exceeded.
    #[error("total timeout of {timeout_secs}s exceeded after {attempts} attempts")]
    Timeout {
        /// Number of attempts made before timeout.
        attempts: u32,
        /// The timeout in seconds.
        timeout_secs: u64,
    },
}

/// A token bucket rate limiter.
///
/// Controls the rate of requests by maintaining a bucket of tokens that
/// are consumed per request and replenished over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    /// Maximum tokens in the bucket.
    pub capacity: u64,
    /// Current number of tokens.
    pub tokens: f64,
    /// Tokens added per second.
    pub refill_rate: f64,
    /// Last time tokens were refilled (epoch millis).
    pub last_refill_ms: u64,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(capacity: u64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill_ms: 0,
        }
    }

    /// Create a rate limiter for a given requests-per-minute.
    pub fn per_minute(rpm: u64) -> Self {
        Self::new(rpm, rpm as f64 / 60.0)
    }

    /// Create a rate limiter for a given requests-per-second.
    pub fn per_second(rps: u64) -> Self {
        Self::new(rps, rps as f64)
    }

    /// Try to consume a token. Returns true if allowed, false if rate-limited.
    pub fn try_acquire(&mut self, now_ms: u64) -> bool {
        self.refill(now_ms);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Try to consume N tokens.
    pub fn try_acquire_n(&mut self, n: u64, now_ms: u64) -> bool {
        self.refill(now_ms);
        let needed = n as f64;
        if self.tokens >= needed {
            self.tokens -= needed;
            true
        } else {
            false
        }
    }

    /// Return the duration to wait until a token is available.
    pub fn wait_duration(&self) -> Duration {
        if self.tokens >= 1.0 {
            return Duration::ZERO;
        }
        let deficit = 1.0 - self.tokens;
        let wait_secs = deficit / self.refill_rate;
        Duration::from_secs_f64(wait_secs)
    }

    /// Return the current fill percentage.
    pub fn fill_percent(&self) -> f64 {
        (self.tokens / self.capacity as f64) * 100.0
    }

    fn refill(&mut self, now_ms: u64) {
        if self.last_refill_ms == 0 {
            self.last_refill_ms = now_ms;
            return;
        }
        let elapsed_secs = (now_ms - self.last_refill_ms) as f64 / 1000.0;
        if elapsed_secs > 0.0 {
            self.tokens = (self.tokens + elapsed_secs * self.refill_rate).min(self.capacity as f64);
            self.last_refill_ms = now_ms;
        }
    }
}

impl fmt::Display for RateLimiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:.0}/{} tokens ({:.1}% full, refill {:.1}/s)",
            self.tokens,
            self.capacity,
            self.fill_percent(),
            self.refill_rate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_backoff() {
        let strategy = BackoffStrategy::exponential();
        assert_eq!(strategy.delay_for_attempt(0), Duration::from_millis(1000));
        assert_eq!(strategy.delay_for_attempt(1), Duration::from_millis(2000));
        assert_eq!(strategy.delay_for_attempt(2), Duration::from_millis(4000));
    }

    #[test]
    fn constant_backoff() {
        let strategy = BackoffStrategy::constant(500);
        assert_eq!(strategy.delay_for_attempt(0), Duration::from_millis(500));
        assert_eq!(strategy.delay_for_attempt(5), Duration::from_millis(500));
    }

    #[test]
    fn rate_limiter_basic() {
        let mut limiter = RateLimiter::per_second(10);
        assert!(limiter.try_acquire(1000));
        assert!(limiter.try_acquire(1000));
    }

    #[test]
    fn retry_policy_status_codes() {
        let policy = RetryPolicy::default_api();
        assert!(policy.should_retry_status(429));
        assert!(policy.should_retry_status(503));
        assert!(!policy.should_retry_status(400));
        assert!(!policy.should_retry_status(401));
    }
}
