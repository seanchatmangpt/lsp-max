//! Per-rule latency tracking for dynamic EvalBudget reclassification.
//!
//! When a Sync-mode rule exceeds 50 ms on 3 consecutive evaluations, it is
//! promoted to Background for the remainder of the session.

use std::collections::VecDeque;
use std::time::Duration;

const WINDOW: usize = 3;
const THRESHOLD: Duration = Duration::from_millis(50);

/// Tracks the last `WINDOW` (3) consecutive execution durations for a Sync rule.
#[derive(Debug, Default)]
pub struct RuleLatencyTracker {
    /// The last up-to-3 observed durations, oldest-first.
    pub recent_durations: VecDeque<Duration>,
    /// True once 3 consecutive evaluations all exceeded the 50 ms threshold.
    pub reclassified: bool,
}

impl RuleLatencyTracker {
    /// Record a new observation. Returns `true` on the first reclassification.
    pub fn record(&mut self, duration: Duration) -> bool {
        if self.reclassified {
            return false;
        }
        if self.recent_durations.len() >= WINDOW {
            self.recent_durations.pop_front();
        }
        self.recent_durations.push_back(duration);

        if self.recent_durations.len() == WINDOW
            && self.recent_durations.iter().all(|d| *d > THRESHOLD)
        {
            self.reclassified = true;
            return true;
        }
        false
    }

    #[inline]
    /// Returns `true` if this rule has been promoted to Background.
    pub fn is_reclassified(&self) -> bool {
        self.reclassified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reclassifies_after_three_slow_evaluations() {
        let mut t = RuleLatencyTracker::default();
        let slow = Duration::from_millis(60);
        assert!(!t.record(slow));
        assert!(!t.record(slow));
        assert!(t.record(slow)); // 3rd slow → reclassified
        assert!(t.is_reclassified());
        assert!(!t.record(slow)); // already reclassified; returns false
    }

    #[test]
    fn does_not_reclassify_on_mixed_durations() {
        let mut t = RuleLatencyTracker::default();
        let fast = Duration::from_millis(10);
        let slow = Duration::from_millis(60);
        t.record(slow);
        t.record(fast); // resets window
        t.record(slow);
        assert!(!t.is_reclassified());
    }
}
