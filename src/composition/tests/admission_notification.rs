//! Gall checkpoint CP10 (see ~/ggen's `80-20-gall-test-refactor-cheerful-quokka.md` plan):
//! proves the one real caller wired in `language_server_impl.rs`'s `ComposedServer::max_admission`
//! override actually reaches a real client (via the real `Client`/`ClientSocket` loopback pair
//! `Client::new` establishes -- the same production machinery `src/service/client/tests.rs`'s
//! `assert_client_message` helper uses, not a hand-rolled mock) when a real admission decision
//! change occurs, mirroring `crates/lsp-max-compositor/tests/integration.rs`'s
//! `compositor_client_deposits_on_publish_diagnostics` shape (build real components, drive a
//! real state change, assert on the real message that came out the other side).
//!
//! Runs against the process-global `REGISTRY`/`get_registry()` singleton
//! (`crate::diagnostics::engine::update_diagnostics`'s law-table scan), which every other
//! `#[cfg(test)] mod tests` unit test in this same `lsp-max` test binary also shares. To avoid
//! interference from any other test's diagnostics, this test forces its verdict transition
//! through a diagnostic keyed under a UUID-suffixed ID no other test could plausibly collide
//! with, and cleans it up (`registry.diagnostics.remove`) before returning -- not relying on
//! process exit for cleanup.

use futures::StreamExt;
use serde_json::Value;

use super::super::server::ComposedServer;
use crate::jsonrpc::Request;
use crate::max_protocol::MaxDiagnostic;
use crate::LanguageServer;
use crate::LspService;
use lsp_types_max::notification::Notification as _;
use lsp_types_max::{Diagnostic, DiagnosticSeverity};

/// `crate::andon::lsp::LspMaxAdmissionChanged`'s real wire method name, hand-transcribed (not
/// re-derived from the notification type itself, to avoid a tautology).
const EXPECTED_METHOD: &str = "lspMax/admissionChanged";

fn unique_diag_id() -> String {
    format!(
        "cp10-test-forced-error-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

#[tokio::test(flavor = "current_thread")]
async fn max_admission_pushes_lsp_max_admission_changed_when_the_verdict_actually_changes() {
    let (service, mut socket) =
        LspService::new(|client| ComposedServer::new(client, Vec::new()));
    let server = service.inner();

    let diag_id = unique_diag_id();

    // Ensure a clean slate: this diagnostic ID cannot exist yet (it's freshly generated).
    {
        let registry = crate::get_registry().lock().unwrap();
        assert!(
            !registry.diagnostics.contains_key(&diag_id),
            "freshly generated diagnostic id must not already exist"
        );
    }

    // Baseline call: whatever the live verdict is right now, record it. This may or may not
    // produce a notification (if it happens to already differ from `state.last_admission_verdict`,
    // which starts at `None`, it always will on the very first call for a fresh `ComposedServer`).
    let baseline = server
        .max_admission()
        .await
        .expect("max_admission must succeed");
    let baseline_verdict = baseline
        .get("verdict")
        .and_then(Value::as_str)
        .expect("verdict field present")
        .to_string();

    // Drain whatever the baseline call produced (there will be exactly one notification, since
    // `last_admission_verdict` starts `None` and the current verdict, whatever it is, is never
    // literally `None`).
    let baseline_req = socket
        .next()
        .await
        .expect("baseline call must push exactly one lspMax/admissionChanged notification");
    assert_eq!(
        baseline_req,
        Request::from_notification::<crate::andon::lsp::LspMaxAdmissionChanged>(
            crate::andon::lsp::AdmissionChangedParams {
                status: baseline_verdict.clone(),
            }
        ),
        "baseline notification must carry the real baseline verdict"
    );
    assert_eq!(
        crate::andon::lsp::LspMaxAdmissionChanged::METHOD,
        EXPECTED_METHOD,
        "sanity: the real Notification::METHOD const must still be lspMax/admissionChanged"
    );

    // Force a real verdict change: insert a real ERROR-severity diagnostic directly into the
    // global registry under our unique id. `max_admission`'s "any ERROR diagnostic => Refused"
    // branch is unconditional on ALL diagnostics present, real production logic
    // (`src/language_server/impls/diagnostics_and_ledger.rs`'s `max_admission`), so this
    // deterministically forces the verdict to "Refused" regardless of what `baseline_verdict` was
    // -- unless `baseline_verdict` was already "Refused", in which case no *second* transition is
    // expected here (asserted below via the actual computed verdict, not an assumption).
    {
        let mut registry = crate::get_registry().lock().unwrap();
        registry.diagnostics.insert(
            diag_id.clone(),
            MaxDiagnostic {
                diagnostic_id: diag_id.clone(),
                lsp: Diagnostic {
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: "CP10 test-forced admission refusal".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
    }

    let forced = server
        .max_admission()
        .await
        .expect("max_admission must succeed after forcing an ERROR diagnostic");
    let forced_verdict = forced
        .get("verdict")
        .and_then(Value::as_str)
        .expect("verdict field present")
        .to_string();
    assert_eq!(
        forced_verdict, "Refused",
        "an ERROR-severity diagnostic present in the registry must force verdict=Refused"
    );

    if forced_verdict != baseline_verdict {
        let forced_req = socket
            .next()
            .await
            .expect("a real verdict change must push a real lspMax/admissionChanged notification");
        assert_eq!(
            forced_req,
            Request::from_notification::<crate::andon::lsp::LspMaxAdmissionChanged>(
                crate::andon::lsp::AdmissionChangedParams {
                    status: "Refused".to_string(),
                }
            ),
            "the notification pushed on the forced transition must carry the real new verdict, \
             not a stale or default one"
        );
    }

    // Revert: remove the forced diagnostic, confirm the verdict goes back to baseline and (if
    // the earlier branch actually ran) a second real notification is pushed for the revert too.
    {
        let mut registry = crate::get_registry().lock().unwrap();
        registry.diagnostics.remove(&diag_id);
    }
    let reverted = server
        .max_admission()
        .await
        .expect("max_admission must succeed after reverting");
    let reverted_verdict = reverted
        .get("verdict")
        .and_then(Value::as_str)
        .expect("verdict field present")
        .to_string();
    assert_eq!(
        reverted_verdict, baseline_verdict,
        "removing the forced diagnostic must restore the original baseline verdict"
    );

    if reverted_verdict != forced_verdict {
        let revert_req = socket
            .next()
            .await
            .expect("the revert transition must also push a real lspMax/admissionChanged notification");
        assert_eq!(
            revert_req,
            Request::from_notification::<crate::andon::lsp::LspMaxAdmissionChanged>(
                crate::andon::lsp::AdmissionChangedParams {
                    status: baseline_verdict,
                }
            ),
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn max_admission_does_not_push_when_the_verdict_is_unchanged() {
    let (service, mut socket) =
        LspService::new(|client| ComposedServer::new(client, Vec::new()));
    let server = service.inner();

    // First call always transitions from `None`, draining it here.
    let first = server.max_admission().await.expect("first call succeeds");
    let first_verdict = first
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    let _ = socket.next().await.expect("first call pushes one notification");

    // A second call with no registry change must compute the SAME verdict and must NOT push a
    // second notification -- proving this is real change-detection, not a blind re-send-on-every-poll.
    let second = server
        .max_admission()
        .await
        .expect("second call succeeds");
    let second_verdict = second.get("verdict").and_then(Value::as_str).unwrap();
    assert_eq!(
        second_verdict, first_verdict,
        "verdict must be stable with no registry change between calls"
    );

    // Race a short timeout against the socket to prove nothing arrives.
    let raced = tokio::time::timeout(std::time::Duration::from_millis(50), socket.next()).await;
    assert!(
        raced.is_err(),
        "no lspMax/admissionChanged notification should be pushed when the verdict did not change, \
         but one arrived: {:?}",
        raced
    );
}
