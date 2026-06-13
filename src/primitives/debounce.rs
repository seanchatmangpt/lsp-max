use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time;

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
