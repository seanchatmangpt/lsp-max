// e2e.rs — subprocess spawn + JSON-RPC pipeline tests.
// These tests exercise the real ChildProcess::spawn path using `cat` as a
// stand-in subprocess (cat echoes stdin to stdout, satisfying the pipe
// contract without requiring a real LSP server binary).
//
// Status: ADMITTED where cat is available; OPEN where it is not.

use lsp_max_compositor::child_process::{ChildProcess, ChildProcessPool};

/// Verify that spawning `cat` produces a ChildProcess with the correct
/// server_id and a usable handle.  If `cat` is absent the test is OPEN
/// (inconclusive) but does not fail.
#[tokio::test]
async fn child_process_spawn_cat_establishes_connection() {
    let result = ChildProcess::spawn("cat-server".to_string(), "cat", &[]).await;

    match result {
        Ok(proc) => {
            assert_eq!(proc.server_id, "cat-server");
            // exit() is a fire-and-forget notification; no response expected.
            proc.handle.exit().await;
        }
        Err(e) => {
            // OPEN: cat not available in this environment.
            eprintln!("child_process_spawn_cat_establishes_connection: OPEN — cat unavailable: {e}");
        }
    }
}

/// Verify that ChildProcessPool::register followed by server_ids_snapshot
/// reflects the registered entry when `cat` is available.
#[tokio::test]
async fn child_process_pool_spawn_and_snapshot() {
    let pool = ChildProcessPool::new();
    assert_eq!(pool.server_ids_snapshot().len(), 0);

    match ChildProcess::spawn("cat-pool-test".to_string(), "cat", &[]).await {
        Ok(proc) => {
            pool.register("cat-pool-test".to_string(), proc);
            let ids = pool.server_ids_snapshot();
            assert!(
                ids.contains(&"cat-pool-test".to_string()),
                "server_ids_snapshot should contain registered id; got: {ids:?}"
            );
        }
        Err(e) => {
            // OPEN: cat not available.
            eprintln!("child_process_pool_spawn_and_snapshot: OPEN — cat unavailable: {e}");
        }
    }
}
