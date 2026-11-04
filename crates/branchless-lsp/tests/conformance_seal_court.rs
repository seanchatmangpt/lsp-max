//! Lane LSPA-1 shutdown-seal court
//! (`docs/jira/v26.11.2/lanes/lane-lspa-01-shutdown-seal.md`).
//!
//! Proves the lib.rs did_save/shutdown wiring end to end with real
//! collaborators: real `LspService` + `Client`/`ClientSocket` loopback
//! (G06 pattern), a real `rustc --emit=obj` compile via `run_tier2`, a
//! real disasm court, and a real 80-byte `SessionConformance::seal()`
//! minted by `shutdown()`. No mocks.
//!
//! Anti-vacuity (the flip): two sessions whose saves carry different
//! court vectors must produce DIFFERENT seal bytes; decoded counters
//! must differ too. No [SKIP] paths: a missing rustc fails loudly via
//! the spawn error, never a fake pass.

use branchless_lsp::conformance::{SessionConformance, SEAL_LEN};
use branchless_lsp::BranchlessLspServer;
use futures::StreamExt;
use lsp_max::jsonrpc::Request;
use lsp_max::{LanguageServer, LspService};
use lsp_types_max::{DidSaveTextDocumentParams, TextDocumentIdentifier};
use tower::{Service, ServiceExt};

/// Real machine-checked branchy fixture (G06 family): a while-loop
/// lowers to a real forward conditional edge at -O on aarch64.
const BRANCHY: &str = "#[no_mangle]\npub extern \"C\" fn g06_branchy(x: u32) -> u32 {\n    let mut i = core::hint::black_box(0u32);\n    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }\n    i\n}\n";

/// Branch-free fixture: no conditional control flow at all.
const CLEAN: &str = "#[no_mangle]\npub extern \"C\" fn g06_clean(x: u32) -> u32 {\n    core::hint::black_box(x).wrapping_add(1)\n}\n";

/// Run `initialize` + the given saves through the REAL service and return
/// it (the server borrows from the service, so the service must outlive).
async fn session_with_saves(saves: &[(&str, &str)]) -> LspService<BranchlessLspServer> {
    let (mut service, mut socket) = LspService::new(BranchlessLspServer::new);
    // The loopback socket is a capacity-1 mpsc (lsp-max
    // src/service/client/mod.rs:46): a second undrained
    // publishDiagnostics blocks the server forever. Drain it for the
    // whole session on a spawned task (witnessed: an undrained socket
    // wedged the 2-save session >25 min at 0 CPU).
    tokio::spawn(async move { while let Some(_req) = socket.next().await {} });
    let init = Request::build("initialize")
        .params(serde_json::json!({"capabilities": {}}))
        .id(1)
        .finish();
    let resp = service
        .ready()
        .await
        .expect("service ready")
        .call(init)
        .await
        .expect("initialize must succeed");
    assert!(resp.is_some(), "initialize must answer");
    for (uri_str, content) in saves {
        let uri: lsp_types_max::Uri = uri_str.parse().expect("valid uri");
        let server = service.inner();
        server
            .did_save(DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
                text: Some((*content).to_string()),
            })
            .await;
    }
    service
}

/// COURT (seal arm): after a real did_save court run, shutdown mints a
/// non-zero 80-byte seal that decodes to the session totals exactly.
#[tokio::test(flavor = "current_thread")]
async fn shutdown_seals_a_nonzero_session_receipt() {
    let service = session_with_saves(&[("file:///tmp/lspa1-a.rs", BRANCHY)]).await;
    let server = service.inner();
    let totals_before = {
        let s = server.session_conformance().lock().expect("lock");
        (
            s.n_admitted,
            s.n_refused,
            s.n_residual,
            s.n_alu,
            s.n_load_store,
        )
    };
    assert!(
        totals_before.0 + totals_before.1 + totals_before.2 >= 1,
        "[FAIL] session totals must fold the did_save court run; got {totals_before:?}"
    );
    server.shutdown().await.expect("shutdown");
    let seal = server
        .tier2_state()
        .lock()
        .expect("lock")
        .session_seal
        .expect("shutdown must mint the 80-byte session seal");
    assert_eq!(seal.len(), SEAL_LEN);
    // The BLAKE3 tag (last 32 bytes) is never all-zero.
    assert!(
        seal[48..80].iter().any(|&b| b != 0),
        "[FAIL] seal digest tag must be non-zero"
    );
    let decoded = SessionConformance::from_seal(&seal).expect("seal decodes");
    assert_eq!(decoded.n_admitted, totals_before.0);
    assert_eq!(decoded.n_refused, totals_before.1);
    assert_eq!(decoded.n_residual, totals_before.2);
    assert_eq!(decoded.n_alu, totals_before.3);
    assert_eq!(decoded.n_load_store, totals_before.4);
}

/// COURT (state arm): the session totals are observable WITHOUT shutdown
/// via the public accessor, and the running count includes every save.
#[tokio::test(flavor = "current_thread")]
async fn session_totals_observable_before_shutdown() {
    let service = session_with_saves(&[
        ("file:///tmp/lspa1-b1.rs", CLEAN),
        ("file:///tmp/lspa1-b2.rs", CLEAN),
    ])
    .await;
    let server = service.inner();
    let s = server.session_conformance().lock().expect("lock");
    assert_eq!(
        s.n_admitted + s.n_residual,
        2,
        "[FAIL] two clean saves must admit exactly 2 fns into the session totals"
    );
    assert_eq!(s.n_refused, 0, "[FAIL] clean saves refuse nothing");
}

/// ANTI-VACUITY (the flip): two sessions with different save vectors
/// (branchy vs clean, a real rustc compile each) must yield DIFFERENT
/// seal bytes, and the decoded counters must differ too.
#[tokio::test(flavor = "current_thread")]
async fn different_save_vectors_flip_the_seal_bytes() {
    let sa_service = session_with_saves(&[("file:///tmp/lspa1-fa.rs", BRANCHY)]).await;
    let sb_service = session_with_saves(&[("file:///tmp/lspa1-fb.rs", CLEAN)]).await;
    let (sa, sb) = {
        let a = sa_service.inner();
        let b = sb_service.inner();
        a.shutdown().await.expect("shutdown a");
        b.shutdown().await.expect("shutdown b");
        let sa = a
            .tier2_state()
            .lock()
            .expect("lock")
            .session_seal
            .expect("seal a");
        let sb = b
            .tier2_state()
            .lock()
            .expect("lock")
            .session_seal
            .expect("seal b");
        (sa, sb)
    };
    assert_ne!(
        sa, sb,
        "[FAIL] different session vectors must produce different seal bytes"
    );
    let da = SessionConformance::from_seal(&sa).expect("decode a");
    let db = SessionConformance::from_seal(&sb).expect("decode b");
    assert_ne!(da, db, "decoded counters must differ too");
}

/// COURT (empty arm): a session with no saves still mints the zeroed
/// framing seal at shutdown and it decodes to a zeroed accumulator.
#[tokio::test(flavor = "current_thread")]
async fn empty_session_seals_zeroed_framing() {
    let service = session_with_saves(&[]).await;
    service.inner().shutdown().await.expect("shutdown");
    let seal = service
        .inner()
        .tier2_state()
        .lock()
        .expect("lock")
        .session_seal
        .expect("empty session must still mint a seal");
    let decoded = SessionConformance::from_seal(&seal).expect("zero seal decodes");
    assert_eq!(decoded, SessionConformance::new());
}
