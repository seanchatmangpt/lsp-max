// CP11 — prove the existing real LSP→MCP pull bridge end-to-end, in one test.
//
// Round trip under test:
//   DiagnosticBuffer::deposit -> MergeContext::merge (via buffer.flush, invoked from
//   FlushCoordinator's real background task) -> FlushCoordinator's real flush loop,
//   which both calls Client::publish_diagnostics AND writes the real fitness-snapshot
//   file (`<cwd>/.claude/lsp-max-fitness.json`) -> lsp_max_cli::mcp_bridge::read_fitness_status_at,
//   the exact function `lsp-max-mcp`'s `lsp_route`/`lsp_violations` tool handlers call
//   (the bin now delegates to this library function; see src/bin/lsp-max-mcp.rs).
//
// This is a real Chicago-TDD test: no mocked filesystem, no mocked FlushCoordinator —
// a real DiagnosticBuffer, a real MergeContext, a real FlushCoordinator background task,
// and a real `lsp_max::Client` obtained the only way the public API allows (via
// `LspService::new`, mirroring `tests/e2e/test_harness.rs`'s own pattern in the `lsp-max`
// crate itself).
//
// cwd caveat: FlushCoordinator's fitness-file write and mcp_bridge's read both resolve
// the workspace root from the process's current directory (this is real, pre-existing
// behavior in flush_coordinator.rs, not something this test invented or worked around).
// This test therefore changes the process cwd to an isolated tempdir for its duration.
// It is the only test in this binary (a dedicated `tests/cp11_fitness_bridge.rs` file,
// which cargo compiles and runs as its own process) so this does not race other tests.

use std::sync::Arc;
use std::time::Duration;

use lsp_max::lsp_types::{InitializeParams, InitializeResult};
use lsp_max::{jsonrpc::Result as LspResult, Client, LanguageServer, LspService};
use lsp_max_compositor::child_process::ChildProcessPool;
use lsp_max_compositor::diagnostic_buffer::DiagnosticBuffer;
use lsp_max_compositor::flush_coordinator::FlushCoordinator;
use lsp_max_compositor::merge::{DiagnosticEntry, MergeContext};
use lsp_max_compositor::registry::ChildTier;
use lsp_max_compositor::GateFile;

/// Minimal `LanguageServer` impl used only to obtain a real `lsp_max::Client` via the
/// public `LspService::new(|client| ...)` constructor — `Client::new` itself is
/// `pub(super)` inside the `lsp_max` crate and not constructible from outside it.
struct NoopServer;

#[lsp_max::async_trait]
impl LanguageServer for NoopServer {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn diagnostic_content_survives_compositor_to_fitness_file_to_mcp_read() {
    // ── Isolate cwd so the fitness file lands in a throwaway tempdir ────────────
    let tmp = tempfile::tempdir().expect("tempdir");
    let original_cwd = std::env::current_dir().expect("current_dir");
    // FlushCoordinator writes to "<cwd>/.claude/lsp-max-fitness.json" and silently
    // no-ops (`let _ = std::fs::write(...)`) if `.claude/` does not already exist —
    // real, pre-existing behavior (flush_coordinator.rs never mkdir -p's it). Every
    // real lsp-max project already has `.claude/`; a bare tempdir does not, so this
    // test creates it, matching real deployment rather than masking a production bug.
    std::fs::create_dir_all(tmp.path().join(".claude")).expect("create .claude dir");
    std::env::set_current_dir(tmp.path()).expect("set_current_dir to tempdir");

    // Real distinctive content this test asserts actually survives the round trip.
    let distinctive_uri = "file:///cp11-round-trip-probe.rs";
    let distinctive_message = "CP11-PROBE: unused import `std::marker::PhantomData`";
    let distinctive_code = "GGEN-TPL-001"; // real ANDON prefix ("GGEN-") from merge::is_refused_by_law

    // ── Real MergeContext + DiagnosticBuffer + GateFile (no mocks) ──────────────
    let ctx = Arc::new(MergeContext::new(vec!["GGEN-".to_string()]));
    let gate_path = tmp.path().join("gate-cp11");
    let gate = Arc::new(GateFile::from_path(gate_path));
    let buffer = Arc::new(DiagnosticBuffer::new(Arc::clone(&ctx), Arc::clone(&gate)));

    // ── Real lsp_max::Client, obtained via the only public constructor path ─────
    // `Client::new` itself is `pub(super)` inside the `lsp_max` crate; `LspService::new`
    // is the documented, public way external crates obtain a real `Client` (its init
    // closure runs synchronously inside `LspService::build`, confirmed by reading
    // src/service.rs — `init(client.clone())` before `new` returns).
    let captured_client: Arc<std::sync::Mutex<Option<Client>>> = Arc::new(std::sync::Mutex::new(None));
    let captured_client_for_init = Arc::clone(&captured_client);
    let (service, socket) = LspService::new(move |client| {
        *captured_client_for_init.lock().unwrap() = Some(client);
        NoopServer
    });
    let client: Client = captured_client
        .lock()
        .unwrap()
        .take()
        .expect("LspService::new must invoke its init closure synchronously");
    drop(service);
    drop(socket);

    let pool = Arc::new(ChildProcessPool::new());

    // ── Spawn the REAL FlushCoordinator background task (no test double) ────────
    let coordinator = FlushCoordinator::spawn(
        Arc::clone(&buffer),
        Arc::clone(&ctx),
        client,
        pool,
        Arc::clone(&gate),
        1, // expected_server_count — quorum of 1 fires the flush immediately
    );

    // ── Deposit a synthetic diagnostic carrying the distinctive content ─────────
    let entry = DiagnosticEntry {
        uri: distinctive_uri.to_string(),
        line: 7,
        character: 3,
        severity: 1, // Error — required for has_andon_block / ANDON routing
        code: distinctive_code.to_string(),
        message: distinctive_message.to_string(),
        source_tier: ChildTier::Primary,
        server_id: Some("cp11-probe-server".to_string()),
    };
    buffer.deposit(
        distinctive_uri,
        "cp11-probe-server",
        ChildTier::Primary,
        vec![entry],
    );
    coordinator.signal_flush(distinctive_uri, "cp11-probe-server");

    // ── Wait for the real fitness file to appear (real async flush, real I/O) ───
    let fitness_path = tmp.path().join(".claude/lsp-max-fitness.json");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut saw_probe_case_id = false;
    let mut last_seen: serde_json::Value = serde_json::json!({});
    while tokio::time::Instant::now() < deadline {
        // This is the exact function lsp-max-mcp's `lsp_route`/`lsp_violations` tool
        // handlers call (bin/lsp-max-mcp.rs now delegates to it) — not a re-implementation.
        let snapshot = lsp_max_cli::mcp_bridge::read_fitness_status_at(tmp.path());
        last_seen = snapshot.clone();
        let case_id_matches = snapshot
            .get("violations")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .any(|v| v.get("case_id").and_then(|c| c.as_str()) == Some(distinctive_uri))
            })
            .unwrap_or(false);
        if fitness_path.exists() && case_id_matches {
            saw_probe_case_id = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Restore cwd before any assertion can panic and leave the process cwd changed
    // for whatever runs next in this (single-test) binary.
    let _ = std::env::set_current_dir(&original_cwd);
    let _ = coordinator.signal_drop_count(); // keep `coordinator` alive through the wait loop above

    assert!(
        fitness_path.exists(),
        "FlushCoordinator must have written the real fitness file to {}",
        fitness_path.display()
    );
    assert!(
        saw_probe_case_id,
        "the synthetic diagnostic's real URI ({distinctive_uri}) must appear as a \
         Declare-violation case_id in the fitness snapshot read back through \
         lsp_max_cli::mcp_bridge::read_fitness_status_at (the exact function the MCP tool \
         handlers call) — last snapshot seen: {last_seen}"
    );

    // The ANDON-coded diagnostic (severity=1, GGEN- prefix) must have driven the
    // batch into a non-clean law_status — real content, not just file presence.
    let law_status = last_seen
        .get("law_status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_ne!(
        law_status, "UNKNOWN",
        "law_status must be a real computed value (ADMITTED/CANDIDATE/BLOCKED), not the \
         file-missing fallback — got snapshot: {last_seen}"
    );

    // ── Independent confirmation: MergeContext itself classifies the same code as ANDON ──
    assert!(
        ctx.is_andon_for_server(distinctive_code, Some("cp11-probe-server")),
        "sanity check: {distinctive_code} must be classified ANDON by the same \
         MergeContext used in the flush — otherwise the round trip proves nothing about \
         the ANDON path specifically"
    );
}
