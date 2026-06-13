// e2e.rs — subprocess spawn + JSON-RPC pipeline tests.
// Uses `lsp-echo-server` (a minimal LSP-speaking binary compiled from
// src/bin/lsp_echo_server.rs) as the subprocess stand-in.  The binary
// responds to `initialize` with `{"capabilities":{}}`, `shutdown` with null,
// and exits on `exit`.

use lsp_max_compositor::child_process::{ChildProcess, ChildProcessPool};

/// Path to the lsp-echo-server test helper binary, resolved at compile time.
const LSP_ECHO_SERVER: &str = env!("CARGO_BIN_EXE_lsp-echo-server");

/// Verify that spawning `lsp-echo-server` produces a ChildProcess with the
/// correct server_id and a usable handle.
#[tokio::test]
async fn child_process_spawn_echo_server_establishes_connection() {
    let result =
        ChildProcess::spawn("echo-server".to_string(), LSP_ECHO_SERVER, &[]).await;

    match result {
        Ok((proc, _exit_fut)) => {
            assert_eq!(proc.server_id, "echo-server");
            proc.handle.exit().await;
        }
        Err(e) => {
            eprintln!(
                "child_process_spawn_echo_server_establishes_connection: BLOCKED — spawn failed: {e}"
            );
            panic!("spawn failed: {e}");
        }
    }
}

/// Verify that ChildProcessPool::register followed by server_ids_snapshot
/// reflects the registered entry.
#[tokio::test]
async fn child_process_pool_spawn_and_snapshot() {
    let pool = ChildProcessPool::new();
    assert_eq!(pool.server_ids_snapshot().len(), 0);

    match ChildProcess::spawn("echo-pool-test".to_string(), LSP_ECHO_SERVER, &[]).await {
        Ok((proc, _exit_fut)) => {
            pool.register("echo-pool-test".to_string(), proc);
            let ids = pool.server_ids_snapshot();
            assert!(
                ids.contains(&"echo-pool-test".to_string()),
                "server_ids_snapshot should contain registered id; got: {ids:?}"
            );
        }
        Err(e) => {
            panic!("spawn failed: {e}");
        }
    }
}
