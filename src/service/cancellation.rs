//! Utilities for ensuring cancellation safety and resource cleanup.

/// A guard that executes a closure when dropped, unless disarmed.
///
/// This is useful for ensuring that resources are cleaned up even if a future
/// is cancelled (dropped) before it completes.
///
/// See the [cancellation safety documentation](../../docs/CANCELLATION_SAFETY.md) for more details.
#[derive(Debug)]
pub struct CancellationGuard<F: FnOnce()> {
    on_drop: Option<F>,
}

impl<F: FnOnce()> CancellationGuard<F> {
    /// Creates a new guard that will execute the given closure when dropped.
    pub fn new(on_drop: F) -> Self {
        Self {
            on_drop: Some(on_drop),
        }
    }

    /// Disarms the guard, preventing the closure from being executed when dropped.
    pub fn disarm(mut self) {
        self.on_drop = None;
    }
}

impl<F: FnOnce()> Drop for CancellationGuard<F> {
    fn drop(&mut self) {
        if let Some(on_drop) = self.on_drop.take() {
            on_drop();
        }
    }
}

/// A trait for types that can be wrapped in a [`CancellationGuard`].
pub trait OnDrop: Sized {
    /// Wraps this value in a [`CancellationGuard`] that will call the given 
    /// closure when dropped.
    fn on_drop<F: FnOnce()>(self, on_drop: F) -> (Self, CancellationGuard<F>) {
        (self, CancellationGuard::new(on_drop))
    }
}

impl<T> OnDrop for T {}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use super::*;

    #[test]
    fn executes_on_drop() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        {
            let _guard = CancellationGuard::new(move || {
                executed_clone.store(true, Ordering::SeqCst);
            });
        }

        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn does_not_execute_when_disarmed() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        {
            let guard = CancellationGuard::new(move || {
                executed_clone.store(true, Ordering::SeqCst);
            });
            guard.disarm();
        }

        assert!(!executed.load(Ordering::SeqCst));
    }

    #[test]
    fn on_drop_helper() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        {
            let (_val, _guard) = "resource".on_drop(move || {
                executed_clone.store(true, Ordering::SeqCst);
            });
        }

        assert!(executed.load(Ordering::SeqCst));
    }
}
