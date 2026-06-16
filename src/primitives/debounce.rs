use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time;

use lsp_types_max::Url;

use super::DocumentStore;

/// Handle to a debounced callback.  Clone is O(1); all clones share the same
/// trigger channel.
///
/// The background task exits automatically when the last `DebounceHandle` is
/// dropped (the watch sender goes away).
#[derive(Clone, Debug)]
pub struct DebounceHandle {
    tx: Arc<watch::Sender<()>>,
}

impl DebounceHandle {
    /// Signal that work is pending.  The callback fires once after `delay`
    /// elapses with no further calls.  Rapid-fire calls reset the timer.
    pub fn trigger(&self) {
        let _ = self.tx.send(());
    }
}

/// Spawn a debounced async callback and return a shareable trigger handle.
///
/// Calling `trigger()` schedules `f` to run after `delay`. If `trigger()` is
/// called again before the delay expires the timer resets — only one `f()`
/// invocation happens per quiet window.
///
/// ```ignore
/// let store = DocumentStore::new();
/// let sink  = DiagnosticSink::new(client.clone());
/// let dbc   = debounce(Duration::from_millis(150), move || {
///     let store = store.clone();
///     let sink  = sink.clone();
///     async move { analyze_and_publish(&store, &sink).await }
/// });
///
/// // In did_change:
/// dbc.trigger();
/// ```
pub fn debounce<F, Fut>(delay: Duration, f: F) -> DebounceHandle
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (tx, mut rx) = watch::channel(());
    let handle = DebounceHandle { tx: Arc::new(tx) };

    tokio::spawn(async move {
        loop {
            // Block until the first trigger arrives.
            if rx.changed().await.is_err() {
                break;
            }
            // Drain rapid-fire triggers within the quiet window.
            while let Ok(Ok(())) = time::timeout(delay, rx.changed()).await {}
            f().await;
        }
    });

    handle
}

/// Spawn a debounced callback whose quiet-window scales with the document's
/// activation density ρ_act.
///
/// High-activation documents (many rapid edits) benefit from a longer quiet
/// window: batching more invalidations into one re-analysis pass reduces
/// redundant work.  The scaling formula is:
///
/// ```text
/// delay = base_delay × clamp(√(activations / 10), 1.0, 8.0)
/// ```
///
/// At `activations = 0` the multiplier is 1.0 (same as plain `debounce`).
/// At `activations = 10` the multiplier is 1.0; at 40 it is 2.0; at 640 it
/// saturates at 8.0 (8 × base_delay maximum).
///
/// `store` and `uri` are read at trigger time (not at spawn time), so the
/// activation count reflects accumulated edits up to that point.
#[tracing::instrument(
    name = "debounce_adaptive",
    skip(store, f),
    fields(
        uri = ?uri,
        base_delay_ms = base_delay.as_millis(),
        activations = tracing::field::Empty,
        multiplier = tracing::field::Empty,
    )
)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };

    #[tokio::test]
    #[ignore]
    async fn debounce_fires_callback_after_quiet_window() {
        let counter = Arc::new(AtomicU64::new(0));
        let c = Arc::clone(&counter);
        let handle = debounce(Duration::from_millis(20), move || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });
        handle.trigger();
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    #[ignore]
    async fn debounce_coalesces_rapid_triggers_into_one_call() {
        let counter = Arc::new(AtomicU64::new(0));
        let c = Arc::clone(&counter);
        let handle = debounce(Duration::from_millis(30), move || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });
        for _ in 0..5 {
            handle.trigger();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "five rapid triggers must coalesce into one callback"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn debounce_handle_clone_shares_trigger_channel() {
        let counter = Arc::new(AtomicU64::new(0));
        let c = Arc::clone(&counter);
        let handle = debounce(Duration::from_millis(20), move || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });
        let handle2 = handle.clone();
        handle2.trigger();
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

pub fn debounce_adaptive<F, Fut>(
    store: DocumentStore,
    uri: Url,
    base_delay: Duration,
    f: F,
) -> DebounceHandle
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (tx, mut rx) = watch::channel(());
    let handle = DebounceHandle { tx: Arc::new(tx) };

    let span = tracing::Span::current();
    tokio::spawn(async move {
        loop {
            if rx.changed().await.is_err() {
                break;
            }
            let acts = store.activation_count(&uri);
            let multiplier = (acts as f64 / 10.0).sqrt().clamp(1.0, 8.0);
            span.record("activations", acts);
            span.record("multiplier", multiplier);
            let delay = base_delay.mul_f64(multiplier);
            while let Ok(Ok(())) = time::timeout(delay, rx.changed()).await {}
            f().await;
        }
    });

    handle
}
