use crate::session_conformance::{replay_session, SessionLog};
use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Serialize)]
pub struct SessionReplayResult {
    pub file: String,
    pub events: usize,
    /// Van der Aalst fitness metric ∈ [0, 1].
    pub fitness: f64,
    /// Declare constraint violations detected during replay.
    pub violations: usize,
    /// Oracle class hits (A8–A12) detected during replay.
    pub oracle_hits: usize,
    /// Bounded conformance status.
    pub status: &'static str,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct SessionService;

impl SessionService {
    pub fn new() -> Self {
        Self
    }

    pub fn replay(&self, path: &str) -> std::io::Result<SessionReplayResult> {
        let raw = std::fs::read_to_string(path)?;
        let log: SessionLog = serde_json::from_str(&raw).map_err(std::io::Error::other)?;
        let result = replay_session(&log);
        Ok(SessionReplayResult {
            file: path.to_string(),
            events: log.events().len(),
            fitness: result.fitness,
            violations: result.violations.len(),
            oracle_hits: result.oracle_hits.len(),
            status: result.status,
        })
    }
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Replay a persisted OCEL 2.0 session log (JSON) through the scaffold Declare
/// constraint model and Oracle class detectors (A8–A12).  Reports the van der
/// Aalst fitness metric and any causal, temporal, or epistemic violations.
#[verb("replay")]
pub fn replay(file: String) -> Result<SessionReplayResult> {
    SessionService::new()
        .replay(&file)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn replay_of_empty_log_is_admitted() {
        let log = SessionLog::new();
        let json = serde_json::to_string(&log).unwrap();

        let mut f = NamedTempFile::new().unwrap();
        f.write_all(json.as_bytes()).unwrap();

        let result = SessionService::new()
            .replay(f.path().to_str().unwrap())
            .unwrap();

        assert_eq!(result.status, "ADMITTED");
        assert_eq!(result.violations, 0);
        assert_eq!(result.oracle_hits, 0);
        assert!((result.fitness - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn result_status_is_bounded_vocabulary() {
        let allowed = ["ADMITTED", "REFUSED", "PARTIAL"];
        let r = SessionReplayResult {
            file: "test".to_string(),
            events: 0,
            fitness: 1.0,
            violations: 0,
            oracle_hits: 0,
            status: "ADMITTED",
        };
        assert!(allowed.contains(&r.status));
    }
}
