//! Lane G06 did_save court (`docs/jira/v26.11.2/lanes/lane-g06-lsp-didsave-court.md`).
//!
//! Proves the lib.rs did_save wiring end to end with real collaborators:
//! a real `LspService` + real `Client`/`ClientSocket` loopback pair, a real
//! `rustc --emit=obj` compile (via `run_tier2`), a real capstone/DWARF
//! disasm court, and a real wire `publishDiagnostics` notification read off
//! the socket. No mocks.
//!
//! Anti-vacuity (the flip): the BRANCHY fixture must publish a
//! severity-ERROR `ANDON_REFUSED` diagnostic naming the offending fn;
//! the CLEAN fixture must publish zero ERROR diagnostics. A court that
//! cannot distinguish these two real compiles is FAIL.
//!
//! [SKIP] paths are loud, never silent: if rustc or the aarch64-apple-darwin
//! target is missing the court fails loudly with the spawn error, it does
//! not skip.

use branchless_lsp::BranchlessLspServer;
use futures::StreamExt;
use lsp_max::jsonrpc::Request;
use lsp_max::{LanguageServer, LspService};
use lsp_types_max::{
    DiagnosticSeverity, DidSaveTextDocumentParams, PublishDiagnosticsParams, TextDocumentIdentifier,
};
use std::time::Duration;
use tower::{Service, ServiceExt};

const DIAG_METHOD: &str = "textDocument/publishDiagnostics";

/// Real machine-checked branchy fixture: a while-loop lowers to a real
/// forward conditional edge at -O on aarch64 (a plain `if` may lower to a
/// branchless csel, which the court correctly admits).
const BRANCHY: &str = "#[no_mangle]\npub extern \"C\" fn g06_branchy(x: u32) -> u32 {\n    let mut i = core::hint::black_box(0u32);\n    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }\n    i\n}\n";

/// Branch-free fixture: no conditional control flow at all.
const CLEAN: &str = "#[no_mangle]\npub extern \"C\" fn g06_clean(x: u32) -> u32 {\n    core::hint::black_box(x).wrapping_add(1)\n}\n";

/// Same branchy body as BRANCHY plus the residual allowance: the full
/// did_save flip in one fixture family.
const ALLOWED: &str = "#[allow_isel_residual]\n#[no_mangle]\npub extern \"C\" fn g06_allowed(x: u32) -> u32 {\n    let mut i = core::hint::black_box(0u32);\n    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }\n    i\n}\n";

/// Drive one did_save through the REAL service and return every
/// publishDiagnostics notification that comes out the loopback socket.
async fn save_and_collect(content: &str, uri_str: &str) -> Vec<PublishDiagnosticsParams> {
    let (mut service, mut socket) = LspService::new(BranchlessLspServer::new);
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

    let server = service.inner();
    let uri: lsp_types_max::Uri = uri_str.parse().expect("valid uri");
    let params = DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        text: Some(content.to_string()),
    };
    server.did_save(params).await;

    // Bounded poll on the observable (socket messages), no fixed sleep.
    let deadline = Duration::from_secs(180);
    let mut pubs = Vec::new();
    // did_save publishes exactly once for the saved URI; stop at the first
    // publish for it (the socket stream never EOFs while the service lives).
    loop {
        match tokio::time::timeout(deadline, socket.next()).await {
            Ok(Some(req)) => {
                if req.method() == DIAG_METHOD {
                    let p: PublishDiagnosticsParams =
                        serde_json::from_value(req.params().cloned().unwrap_or_default())
                            .expect("publishDiagnostics params decode");
                    if p.uri.as_str() == uri_str {
                        pubs.push(p);
                        break;
                    }
                }
            }
            Ok(None) => break,
            Err(_) => break, // deadline: stop polling, assert on what arrived
        }
    }
    pubs
}

fn err_diags(p: &PublishDiagnosticsParams) -> Vec<&lsp_types_max::Diagnostic> {
    p.diagnostics
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect()
}

/// COURT (branchy arm): a save of an un-allowed branchy fn must publish a
/// severity-ERROR ANDON diagnostic carrying the offending fn's name.
#[tokio::test(flavor = "current_thread")]
async fn did_save_branchy_publishes_error_naming_the_function() {
    let uri = "file:///tmp/g06-branchy.rs";
    let pubs = save_and_collect(BRANCHY, uri).await;
    assert!(
        !pubs.is_empty(),
        "[FAIL] did_save must publish diagnostics for the branchy save"
    );
    let errs = err_diags(&pubs[0]);
    assert!(
        !errs.is_empty(),
        "[FAIL] branchy save must publish >=1 ERROR diagnostic; got {} diag(s)",
        pubs[0].diagnostics.len()
    );
    assert!(
        errs.iter().any(|d| d.message.contains("g06_branchy")),
        "[FAIL] the ERROR diagnostic must carry the offending fn name; got {:?}",
        errs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(
        errs.iter().any(|d| d.code
            == Some(lsp_types_max::NumberOrString::String(
                branchless_lsp::rule_pack::ANDON_RULE_ID.into()
            ))),
        "[FAIL] diagnostic must carry the pinned rule id"
    );
}

/// COURT (clean arm = the flip): a save of a branch-free fn must publish
/// zero ERROR diagnostics over the exact same real pipeline.
#[tokio::test(flavor = "current_thread")]
async fn did_save_clean_publishes_no_error() {
    let pubs = save_and_collect(CLEAN, "file:///tmp/g06-clean.rs").await;
    assert!(
        !pubs.is_empty(),
        "[FAIL] did_save must still publish (empty) diagnostics for a clean save"
    );
    assert!(
        err_diags(&pubs[0]).is_empty(),
        "[FAIL] clean save must publish zero ERROR diagnostics; got {:?}",
        pubs[0]
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

/// COURT (allowance arm): the SAME branchy body annotated with
/// #[allow_isel_residual] must publish zero ERROR diagnostics — the
/// refusal flips off with the allowance, proving the court is not
/// keying on mere syntax.
#[tokio::test(flavor = "current_thread")]
async fn did_save_allowed_branchy_flips_to_clean() {
    let pubs = save_and_collect(ALLOWED, "file:///tmp/g06-allowed.rs").await;
    assert!(
        !pubs.is_empty(),
        "[FAIL] did_save must publish for the allowed save"
    );
    assert!(
        err_diags(&pubs[0]).is_empty(),
        "[FAIL] allowed branchy save must publish zero ERROR diagnostics; got {:?}",
        pubs[0]
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

/// COURT (state arm): after a refusing save, the live Tier2State and the
/// wire max/conformanceVector must reflect the refusal (n_refused >= 1).
#[tokio::test(flavor = "current_thread")]
async fn did_save_updates_live_tier2_state_and_conformance() {
    let uri_str = "file:///tmp/g06-state.rs";
    let (mut service, _socket) = LspService::new(BranchlessLspServer::new);
    let init = Request::build("initialize")
        .params(serde_json::json!({"capabilities": {}}))
        .id(1)
        .finish();
    let _ = service
        .ready()
        .await
        .expect("service ready")
        .call(init)
        .await
        .expect("initialize");
    let server = service.inner();
    let uri: lsp_types_max::Uri = uri_str.parse().unwrap();
    server
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            text: Some(BRANCHY.to_string()),
        })
        .await;
    {
        let st = server.tier2_state().lock().expect("tier2 lock");
        assert_eq!(
            st.last_uri.as_deref(),
            Some(uri_str),
            "tier2 state must record the saved uri"
        );
        let v = st
            .last_vector
            .as_ref()
            .expect("tier2 state must record the verdict vector");
        assert!(
            v.n_refused >= 1,
            "vector must count the refusal; vector: n_refused={}",
            v.n_refused
        );
    } // guard ends lexically before the await below
    let cv = server
        .max_conformance_vector(None)
        .await
        .expect("conformance vector");
    assert!(
        cv.refused
            .iter()
            .any(|a| format!("{a:?}").contains("ANDON_REFUSED")),
        "wire conformance vector must carry the ANDON_REFUSED axis; got {:?}",
        cv.refused
    );
}

// ===========================================================================
// Lane LSPA-3 (inventory #14 close): live-wire arms for the parameterized
// #[allow_isel_residual(target = "...", max_branches = N)] form.
//
// Until this block, the target/max_branches gate (attributes.rs:77-93,
// target_matches + ResidualParams::classify_count) had never run against a
// real `rustc --emit=obj` object on the did_save wire: the ALLOWED fixture
// above uses the BARE attribute, which takes the classify() bool path and
// never exercises target matching or the exact-count budget.
//
// Measurement law (dispatch: "do not guess N"): N is MEASURED, not
// hardcoded. Each arm first runs the real Tier2 court (real rustc compile +
// capstone disasm) over the bare BRANCHY fixture and reads the actual
// per-function branch-site count from the verdict. The parameterized
// allowance is then built from that measured value.
//
// Expected semantics under the current implementation:
//  - target "aarch64" prefix-matches the court's target_str "aarch64"
//    (TargetKind::Aarch64AppleDarwin -> "aarch64", rule_pack.rs:238) ->
//    classify_seeded_count honors the budget;
//  - max_branches = N   -> HintResidual (1 <= N) -> zero ERROR;
//  - max_branches = N-1 -> Some(1) <= Some(N-1) is false when N == 1, and
//    sites.len() > N-1 otherwise -> ErrorRefused -> ERROR survives;
//  - target "riscv32imc" mismatches "aarch64" -> the parameterized form does
//    NOT set the bare flag (attributes.rs:115-119), so the allowance falls
//    through seed-grid lookup (g06_* is unseeded) to classify_count with
//    bare=false -> ErrorRefusedNoAttr -> typed refusal. If this arm
//    SILENTLY PASSES the finding is surfaced by the assert, not hidden.
use branchless_lsp::compiler_worker::TargetKind;
use branchless_lsp::rule_pack::run_tier2;

/// Run the REAL Tier2 court over the bare branchy fixture and return the
/// measured per-function branch-site count for `g06_branchy`. This is the
/// dispatch-mandated measurement step: no guessed budgets.
fn measured_branch_count() -> usize {
    let out = run_tier2(BRANCHY, TargetKind::Aarch64AppleDarwin);
    let f = out
        .functions
        .iter()
        .find(|f| f.name == "g06_branchy")
        .expect("court must classify g06_branchy");
    let n = f.branch_sites.len();
    assert!(
        n >= 1,
        "[FAIL] measured branch count must be >=1 (the fixture must be \
         branchy on the real object); got {}",
        n
    );
    n
}

/// Build the branchy body carrying a parameterized allowance.
fn allowed_with(target: &str, max_branches: usize) -> String {
    format!(
        "#[allow_isel_residual(target = \"{}\", max_branches = {}, \
         reason = \"lspa-3 measured budget\")]\n{}",
        target, max_branches, BRANCHY
    )
}

/// Shared wire arm: did_save -> publish over the real tower loopback
/// (reuses save_and_collect verbatim) and return the ERROR diagnostics.
async fn wire_errors(content: String, uri_str: &str) -> Vec<lsp_types_max::Diagnostic> {
    let pubs = save_and_collect(&content, uri_str).await;
    assert!(
        !pubs.is_empty(),
        "[FAIL] did_save must publish diagnostics for the parameterized save"
    );
    err_diags(&pubs[0]).into_iter().cloned().collect::<Vec<_>>()
}

/// ARM (i): measured budget N admits the branchy fn — zero ERROR on the
/// wire. Proves target_matches + classify_count run against a real rustc
/// object and that an exact, measured budget is honored.
#[tokio::test(flavor = "current_thread")]
async fn lspa3_measured_budget_admits_branchy_on_wire() {
    let n = measured_branch_count();
    let errs = wire_errors(allowed_with("aarch64", n), "file:///tmp/lspa3-admit.rs").await;
    assert!(
        errs.is_empty(),
        "[FAIL] measured budget max_branches={} must admit the branchy fn \
         (zero ERROR); got {:?}",
        n,
        errs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// ARM (ii), the anti-vacuity flip: budget N-1 refuses. Same fixture, same
/// real pipeline, one less allowed branch -> the ERROR must survive.
#[tokio::test(flavor = "current_thread")]
async fn lspa3_budget_minus_one_refuses_branchy_on_wire() {
    let n = measured_branch_count();
    let errs = wire_errors(
        allowed_with("aarch64", n - 1),
        "file:///tmp/lspa3-refuse.rs",
    )
    .await;
    assert!(
        !errs.is_empty(),
        "[FAIL] max_branches={} (measured {} minus one) must refuse the \
         branchy fn; the gate let it pass — vacuous budget",
        n - 1,
        n
    );
    assert!(
        errs.iter().any(|d| d.message.contains("g06_branchy")),
        "[FAIL] the refusal must name the offending fn; got {:?}",
        errs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(
        errs.iter().any(|d| d.code
            == Some(lsp_types_max::NumberOrString::String(
                branchless_lsp::rule_pack::ANDON_RULE_ID.into()
            ))),
        "[FAIL] the refusal must carry the pinned ANDON rule id"
    );
}

/// ARM (iii): target mismatch. `target = "riscv32imc"` under the aarch64
/// court must be a typed refusal (the parameterized form does not set the
/// bare flag, so mismatched-target fallback keeps refusing semantics).
/// If the mismatch silently passes, this assert fails loudly — FINDING.
#[tokio::test(flavor = "current_thread")]
async fn lspa3_target_mismatch_is_typed_refusal_on_wire() {
    let n = measured_branch_count();
    let errs = wire_errors(
        allowed_with("riscv32imc", n),
        "file:///tmp/lspa3-mismatch.rs",
    )
    .await;
    assert!(
        !errs.is_empty(),
        "[FINDING] target-mismatch allowance (riscv32imc under the aarch64 \
         court) silently passed with max_branches={} — the mismatched \
         parameterized allowance leaked through target_matches fallback",
        n
    );
    assert!(
        errs.iter().any(|d| d.message.contains("g06_branchy")),
        "[FAIL] mismatch refusal must name the offending fn; got {:?}",
        errs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}
