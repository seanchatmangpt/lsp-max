# Cancellation Safety

In Rust, futures are cancelled by being dropped. This is a fundamental aspect of the `Future` trait and the async ecosystem. When a future is dropped, it is no longer polled, and its state is cleaned up.

## Cancellation in `lsp-max`

In `lsp-max`, server-side handling of client requests is tracked in the `Pending` map. When a request is started, it is wrapped in a `future::abortable` wrapper, which provides an `AbortHandle`.

1.  A client sends a request.
2.  `lsp-max` starts an asynchronous task to handle the request.
3.  The request ID and its `AbortHandle` are stored in the `Pending` map.
4.  If the client sends a `$/cancelRequest` notification with that ID:
    - The `AbortHandle` is used to abort the future.
    - The future is immediately dropped.
    - The task resolves to a "request cancelled" error response.

## Why Cancellation Safety Matters

Because cancellation is just "dropping the future," any logic that follows an `.await` point might never execute if the future is cancelled at that point.

Consider this example:

```rust
async fn handle_request() -> Result<(), Error> {
    let temp_file = create_temp_file().await?;
    
    // If the future is cancelled during this .await, 
    // the code below will NEVER run.
    do_some_work().await?;
    
    // This cleanup might be skipped!
    remove_temp_file(temp_file).await?;
    Ok(())
}
```

If `do_some_work().await` is cancelled, `remove_temp_file` is never called.

## Ensuring Cleanup with `CancellationGuard`

To ensure resources are released even when a future is cancelled (dropped), use the RAII (Resource Acquisition Is Initialization) pattern. In Rust, this means implementing the `Drop` trait.

`lsp-max` provides a `CancellationGuard` helper to make this easier.

### Example: Basic Cleanup

```rust
use lsp_max::service::CancellationGuard;

async fn handle_request() -> Result<(), Error> {
    let temp_file = create_temp_file().await?;
    
    // This guard will run its closure when dropped,
    // unless it is explicitly disarmed.
    let guard = CancellationGuard::new(|| {
        println!("Cleaning up temp file...");
        let _ = std::fs::remove_file("temp.txt");
    });

    do_some_work().await?;

    // If we reached here, we might want to "disarm" the guard
    // if the cleanup should only happen on failure/cancellation.
    // Or just let it drop naturally if it should always run.
    guard.disarm(); 
    
    Ok(())
}
```

### Example: Using the `OnDrop` helper

The `OnDrop` trait provides a convenient `.on_drop()` method for any type.

```rust
use lsp_max::service::OnDrop;

async fn handle_request() -> Result<(), Error> {
    let (temp_file, _guard) = create_temp_file().await?.on_drop(|| {
        let _ = std::fs::remove_file("temp.txt");
    });

    do_some_work().await?;

    // The temp_file and its guard will be dropped at the end of the scope.
    Ok(())
}
```

## Best Practices

1.  **Prefer RAII:** Wrap resources in types that implement `Drop`.
2.  **Keep Drop Simple:** `Drop::drop` is synchronous and should not block for long periods.
3.  **Async Cleanup:** If you need to perform asynchronous cleanup on drop, consider spawning a new background task:
    ```rust
    let guard = CancellationGuard::new(|| {
        tokio::spawn(async move {
            cleanup_async().await;
        });
    });
    ```
4.  **Idempotence:** Ensure your cleanup logic is idempotent, as it might be called in various states of completion.
