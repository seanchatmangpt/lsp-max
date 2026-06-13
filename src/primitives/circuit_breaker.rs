//! Circuit breaker protecting the rule-evaluation loop.
//!
//! State machine (ported from wasm4pm/src/self_healing.rs):
//!
//!   Closed ──(3 failures)──► Open
//!   Open   ──(cooldown)────► HalfOpen
//!   HalfOpen ─(success)───► Closed
//!   HalfOpen ─(failure)───► Open

use std::time::{Duration, Instant};

/// Circuit state. `#[repr(u8)]` matches the wasm4pm encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Normal operation — requests flow through.
    Closed = 0,
    /// Recovery probe — one test request allowed through.
    HalfOpen = 1,
    /// Tripped — requests are rejected until cooldown elapses.
    Open = 2,
}

/// Circuit breaker for the rule-evaluation loop.
///
/// Wrap in `Arc<parking_lot::Mutex<CircuitBreaker>>` when sharing across tasks.
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    cooldown: Duration,
    failure_threshold: u32,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    /// Create with default thresholds (3 failures, 30 s cooldown).
    pub fn new() -> Self {
        Self::with_config(3, Duration::from_secs(30))
    }

    /// Create with custom failure threshold and cooldown duration.
    pub fn with_config(failure_threshold: u32, cooldown: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
            cooldown,
            failure_threshold,
        }
    }

    /// Current state (read-only snapshot for logging/metrics).
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Returns `true` when the caller is allowed to attempt the operation.
    /// Transitions Open → HalfOpen when cooldown has elapsed.
    pub fn is_allowed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                let elapsed = self
                    .last_failure
                    .map(|t| t.elapsed())
                    .unwrap_or(Duration::MAX);
                if elapsed >= self.cooldown {
                    tracing::info!(
                        cooldown_elapsed_ms = elapsed.as_millis(),
                        "CircuitBreaker: Open → HalfOpen"
                    );
                    self.state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful operation. Resets to Closed from HalfOpen.
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                tracing::info!("CircuitBreaker: HalfOpen → Closed");
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                self.last_failure = None;
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failure. Trips Open after `failure_threshold` consecutive failures.
    pub fn record_failure(&mut self) -> CircuitState {
        self.failure_count += 1;
        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    tracing::warn!(
                        failure_count = self.failure_count,
                        "CircuitBreaker: Closed → Open"
                    );
                    self.state = CircuitState::Open;
                    self.last_failure = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!("CircuitBreaker: HalfOpen → Open (probe failed)");
                self.state = CircuitState::Open;
                self.last_failure = Some(Instant::now());
            }
            CircuitState::Open => {
                self.last_failure = Some(Instant::now());
            }
        }
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trips_open_after_threshold_failures() {
        let mut cb = CircuitBreaker::with_config(3, Duration::from_secs(3600));
        assert!(cb.is_allowed());
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure(); // 3rd → Open
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn half_open_to_closed_on_success() {
        let mut cb = CircuitBreaker::with_config(1, Duration::from_millis(1));
        cb.record_failure(); // trips Open
        std::thread::sleep(Duration::from_millis(5));
        assert!(cb.is_allowed()); // cooldown elapsed → HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn half_open_to_open_on_failure() {
        let mut cb = CircuitBreaker::with_config(1, Duration::from_millis(1));
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(5));
        cb.is_allowed(); // → HalfOpen
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
