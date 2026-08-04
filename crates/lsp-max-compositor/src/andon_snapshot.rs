use crate::dt_context::{AndonEvent, DtContext, RepairAction};
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;

/// Active event store tracking live ANDON states.
#[derive(Debug, Clone, Default)]
pub struct AndonSnapshot {
    inner: Arc<RwLock<DtContext>>,
}

impl AndonSnapshot {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(DtContext::empty())),
        }
    }

    /// Retrieve a clone of the current D_t context.
    pub fn get_context(&self) -> DtContext {
        self.inner.read().clone()
    }

    /// Replace the current D_t context entirely.
    pub fn set_context(&self, ctx: DtContext) {
        *self.inner.write() = ctx;
    }

    /// Update the current D_t context through a closure.
    pub fn update<F>(&self, mut f: F)
    where
        F: FnMut(&mut DtContext),
    {
        let mut ctx = self.inner.write();
        f(&mut ctx);
    }

    /// Recompute live ANDON states and bump the D_t context sequence.
    pub fn commit_new_state(
        &self,
        active_andon_codes: Vec<String>,
        governing_axes: Vec<String>,
        events: Vec<AndonEvent>,
        repairs: Vec<RepairAction>,
    ) {
        let mut ctx = self.inner.write();

        let mut seq = ctx.seq.unwrap_or(0);
        seq += 1;

        ctx.seq = Some(seq);
        ctx.timestamp = Utc::now().to_rfc3339();

        let has_blocking = events.iter().any(|e| e.blocking);
        ctx.admission_allowed = Some(!has_blocking);

        ctx.active_andon_codes = active_andon_codes;
        ctx.governing_axes = governing_axes;
        ctx.events = events;
        ctx.repairs = repairs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_materializes_blocking_context_and_increments_sequence() {
        let snapshot = AndonSnapshot::new();
        snapshot.commit_new_state(
            vec!["GGEN-TPL-001".to_string()],
            vec!["diagnostic-admission".to_string()],
            vec![AndonEvent {
                code: "GGEN-TPL-001".to_string(),
                blocking: true,
            }],
            vec![RepairAction {
                next_lawful_step: "repair template".to_string(),
                required_command: "ggen sync".to_string(),
            }],
        );

        let first = snapshot.get_context();
        assert_eq!(first.seq, Some(1));
        assert_eq!(first.admission_allowed, Some(false));
        assert_eq!(first.active_andon_codes, vec!["GGEN-TPL-001"]);
        assert!(first.render_gate_context().is_ok());

        snapshot.commit_new_state(Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let second = snapshot.get_context();
        assert_eq!(second.seq, Some(2));
        assert_eq!(second.admission_allowed, Some(true));
        assert!(second.active_andon_codes.is_empty());
    }
}
