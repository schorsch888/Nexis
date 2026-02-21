//! Retry logic for indexing operations

use std::future::Future;
use std::time::Duration;

use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            multiplier: 2.0,
        }
    }
}

pub struct RetryPolicy {
    config: RetryConfig,
    current_attempt: usize,
    current_delay_ms: u64,
}

impl RetryPolicy {
    pub fn new(config: RetryConfig) -> Self {
        Self {
            current_delay_ms: config.initial_delay_ms,
            current_attempt: 0,
            config,
        }
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.current_attempt >= self.config.max_retries {
            return None;
        }

        let delay = Duration::from_millis(self.current_delay_ms);
        self.current_attempt += 1;

        self.current_delay_ms = ((self.current_delay_ms as f64) * self.config.multiplier)
            .min(self.config.max_delay_ms as f64) as u64;

        Some(delay)
    }

    pub fn attempt(&self) -> usize {
        self.current_attempt
    }
}

pub async fn with_retry<F, Fut, T, E>(mut operation: F, config: &RetryConfig) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut policy = RetryPolicy::new(config.clone());

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if let Some(delay) = policy.next_delay() {
                    warn!(
                        attempt = policy.attempt(),
                        delay_ms = delay.as_millis(),
                        error = %e,
                        "Operation failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    debug!(attempts = policy.attempt(), error = %e, "Operation failed after all retries");
                    return Err(e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_policy_delays_increase() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            multiplier: 2.0,
        };

        let mut policy = RetryPolicy::new(config);

        let d1 = policy.next_delay().unwrap();
        assert_eq!(d1, Duration::from_millis(100));

        let d2 = policy.next_delay().unwrap();
        assert_eq!(d2, Duration::from_millis(200));

        let d3 = policy.next_delay().unwrap();
        assert_eq!(d3, Duration::from_millis(400));

        assert!(policy.next_delay().is_none());
    }

    #[test]
    fn retry_policy_respects_max_delay() {
        let config = RetryConfig {
            max_retries: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 2000,
            multiplier: 3.0,
        };

        let mut policy = RetryPolicy::new(config);

        let d1 = policy.next_delay().unwrap();
        assert_eq!(d1, Duration::from_millis(1000));

        let d2 = policy.next_delay().unwrap();
        assert_eq!(d2, Duration::from_millis(2000));

        let d3 = policy.next_delay().unwrap();
        assert_eq!(d3, Duration::from_millis(2000));
    }

    #[tokio::test]
    async fn with_retry_succeeds_after_failures() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            multiplier: 2.0,
        };

        let result = with_retry(
            || {
                let attempts = attempts_clone.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err("temporary failure")
                    } else {
                        Ok("success")
                    }
                }
            },
            &config,
        )
        .await;

        assert_eq!(result, Ok("success"));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn with_retry_fails_after_max_retries() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            multiplier: 2.0,
        };

        let result: Result<&str, &str> = with_retry(|| async { Err("always fails") }, &config).await;

        assert_eq!(result, Err("always fails"));
    }
}
