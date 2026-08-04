//! Gall checkpoint CP4 (see ~/ggen's `80-20-gall-test-refactor-cheerful-quokka.md`
//! plan): applies the CP1/CP2 fidelity-correction + dual-proof pattern to
//! `ontology/lsp318.ttl` instead of `lsif06.ttl`.
//!
//! The question this file answers: for every `lsp:Request`/`lsp:Notification`
//! individual whose `law:status` is `law:ADMITTED`, is that method *actually*
//! registered somewhere in real code today? An `ADMITTED` claim with no real
//! registration is a real bug in the ontology, not a Rust bug -- this test
//! would report it as a finding, not silently patch the ontology to make
//! itself pass.
//!
//! `EXPECTED_REGISTERED` below is not re-derived by hand from a fresh grep of
//! `#[rpc(name = "...")]` in `src/language_server.rs`: it is a **literal copy**
//! of `crate::coverage::lsp_coverage::IMPLEMENTED_METHODS`, which that module's
//! own doc comment states is itself grep-verified against
//! `src/language_server.rs`'s real `#[rpc(name = "...")]` attributes
//! (`grep 'rpc(name' src/language_server.rs | grep -oP '(?<=name = ")[^"]+'`).
//! Re-running that exact grep against `src/language_server.rs` on 2026-08-04
//! confirms the two lists still agree (spot-checked by diffing this const
//! against the sorted grep output and `IMPLEMENTED_METHODS` -- all three
//! match). Using the existing const rather than a second hand-transcription
//! avoids two independently-stale copies of the same ground truth ever
//! silently diverging.
//!
//! `oxigraph::*` use here is in bounds per
//! `src/runtime/control_plane/semantic_graph/store.rs`'s own
//! `OXIGRAPH_BOUNDARY_HELD` invariant, which is explicitly scoped to
//! production code ("outside tests").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

const LSP318_TTL: &str = include_str!("../ontology/lsp318.ttl");
/// `law:ADMITTED`/`law:CANDIDATE`/etc. individuals only get an `rdfs:label`
/// (e.g. `"CANDIDATE"`) in this separate file -- `lsp318.ttl` only references
/// the status IRIs, it does not redeclare their labels. Needed only by
/// `select_all_method_status_pairs`'s `rdfs:label` join; the other queries in
/// this file match on the `law:ADMITTED` IRI directly and don't need it.
const LAW_AXES_TTL: &str = include_str!("../ontology/law-axes.ttl");

/// Literal copy of `lsp_max::coverage::lsp_coverage::IMPLEMENTED_METHODS`
/// (see module doc above for why this is a copy, not a re-derivation).
/// Includes standard LSP methods and `max/*` extensions -- both are valid
/// "registered somewhere" evidence for this checkpoint's purposes.
const EXPECTED_REGISTERED: &[&str] = &[
    "initialize",
    "initialized",
    "shutdown",
    "textDocument/didOpen",
    "textDocument/didChange",
    "textDocument/willSave",
    "textDocument/willSaveWaitUntil",
    "textDocument/didSave",
    "textDocument/didClose",
    "textDocument/declaration",
    "textDocument/definition",
    "textDocument/typeDefinition",
    "textDocument/implementation",
    "textDocument/references",
    "textDocument/prepareCallHierarchy",
    "callHierarchy/incomingCalls",
    "callHierarchy/outgoingCalls",
    "textDocument/prepareTypeHierarchy",
    "typeHierarchy/supertypes",
    "typeHierarchy/subtypes",
    "textDocument/documentHighlight",
    "textDocument/documentLink",
    "documentLink/resolve",
    "textDocument/hover",
    "textDocument/codeLens",
    "codeLens/resolve",
    "textDocument/foldingRange",
    "textDocument/selectionRange",
    "textDocument/documentSymbol",
    "textDocument/documentColor",
    "textDocument/colorPresentation",
    "textDocument/linkedEditingRange",
    "textDocument/moniker",
    "textDocument/completion",
    "completionItem/resolve",
    "textDocument/signatureHelp",
    "textDocument/codeAction",
    "codeAction/resolve",
    "textDocument/rename",
    "textDocument/prepareRename",
    "textDocument/formatting",
    "textDocument/rangeFormatting",
    "textDocument/rangesFormatting",
    "textDocument/onTypeFormatting",
    "workspace/symbol",
    "workspaceSymbol/resolve",
    "workspace/executeCommand",
    "workspace/didChangeConfiguration",
    "workspace/didChangeWatchedFiles",
    "workspace/didChangeWorkspaceFolders",
    "workspace/willCreateFiles",
    "workspace/willRenameFiles",
    "workspace/willDeleteFiles",
    "workspace/didCreateFiles",
    "workspace/didRenameFiles",
    "workspace/didDeleteFiles",
    "workspace/textDocumentContent",
    "textDocument/semanticTokens/full",
    "textDocument/semanticTokens/full/delta",
    "textDocument/semanticTokens/range",
    "textDocument/inlayHint",
    "inlayHint/resolve",
    "textDocument/inlineValue",
    "textDocument/inlineCompletion",
    "textDocument/diagnostic",
    "workspace/diagnostic",
    "notebookDocument/didOpen",
    "notebookDocument/didChange",
    "notebookDocument/didSave",
    "notebookDocument/didClose",
    "window/workDoneProgress/cancel",
    "$/setTrace",
    "$/progress",
    // max/* extensions (also #[rpc]-registered in src/language_server.rs)
    "max/snapshot",
    "max/conformanceVector",
    "max/explainDiagnostic",
    "max/repairPlan",
    "max/applyRepairTransaction",
    "max/exportAnalysisBundle",
    "max/runGate",
    "max/clearDiagnostic",
    "max/receipt",
    "max/releaseActuation",
    "max/admission",
    "max/autonomicLoop",
    "max/chain",
    "max/hook",
    "max/hookGraph",
    "max/lawfulTransition",
    "max/ledgerReport",
    "max/manifoldSnapshot",
    "max/propagate",
    "max/refusal",
    "max/replay",
    "max/verifyLedger",
    "max/conformanceDelta",
    "max/dumpState",
    "max/restoreState",
    "max/instanceList",
    "max/reset",
    "max/lsif",
    "max/rulePacks",
    "max/rulePackStatus",
    "max/rulePackDiff",
    "max/workspaceConformance",
    "workspace/executeSparql",
    "max/intent.validate",
];

fn store_from_ttl(ttl: &str) -> Store {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, ttl.as_bytes())
        .expect("lsp318.ttl must parse as valid Turtle");
    store
}

/// Every `lsp:methodName` whose `law:status` is exactly `law:ADMITTED`.
fn select_admitted_method_names(store: &Store) -> BTreeSet<String> {
    let sparql = r#"
        PREFIX lsp: <https://lsp-max.rs/ontology/>
        PREFIX law: <https://lsp-max.rs/law/>
        SELECT ?methodName WHERE {
            ?individual lsp:methodName ?methodName ;
                        law:status law:ADMITTED .
        }
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions
        .map(|s| {
            let s = s.expect("solution");
            s.get("methodName")
                .expect("?methodName bound")
                .to_string()
                .trim_matches('"')
                .to_string()
        })
        .collect()
}

/// Every `lsp:methodName` declared anywhere in the ontology, paired with its
/// `law:status` label (via `rdfs:label` on the status individual).
fn select_all_method_status_pairs(store: &Store) -> Vec<(String, String)> {
    let sparql = r#"
        PREFIX lsp: <https://lsp-max.rs/ontology/>
        PREFIX law: <https://lsp-max.rs/law/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT ?methodName ?statusLabel WHERE {
            ?individual lsp:methodName ?methodName ;
                        law:status ?status .
            ?status rdfs:label ?statusLabel .
        }
        ORDER BY ?methodName
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions
        .map(|s| {
            let s = s.expect("solution");
            let method = s
                .get("methodName")
                .expect("?methodName bound")
                .to_string()
                .trim_matches('"')
                .to_string();
            let status = s
                .get("statusLabel")
                .expect("?statusLabel bound")
                .to_string()
                .trim_matches('"')
                .to_string();
            (method, status)
        })
        .collect()
}

#[test]
fn lsp318_ttl_parses_as_valid_turtle() {
    let _store = store_from_ttl(LSP318_TTL);
}

/// SPARQL-derived proof, hand-transcribed cross-check: every method
/// individual whose `law:status` is `law:ADMITTED` must have its
/// `methodName` present in `EXPECTED_REGISTERED` (the grep-verified real
/// registration list). As of 2026-08-04 this set is EMPTY -- `lsp318.ttl`
/// declares zero `ADMITTED` methods (all 57 request/notification
/// individuals are `law:CANDIDATE`; see
/// `admitted_set_is_currently_empty_this_is_a_real_finding` below for the
/// explicit, non-silent statement of that fact) -- so this assertion
/// currently holds vacuously. It is not a tautology: the drift-injection
/// test below proves the same check logic actually refuses a real
/// ADMITTED-but-unregistered case when one exists.
#[test]
fn every_admitted_method_is_actually_registered() {
    let store = store_from_ttl(LSP318_TTL);
    let admitted = select_admitted_method_names(&store);
    let registered: BTreeSet<&str> = EXPECTED_REGISTERED.iter().copied().collect();

    let unregistered_but_admitted: Vec<&String> = admitted
        .iter()
        .filter(|m| !registered.contains(m.as_str()))
        .collect();

    assert!(
        unregistered_but_admitted.is_empty(),
        "lsp318.ttl claims law:ADMITTED for method(s) with no real #[rpc(...)] \
         registration found anywhere in the workspace: {:?}. An ADMITTED claim \
         with no real registration is an ontology bug -- investigate before \
         changing either side.",
        unregistered_but_admitted
    );
}

/// Explicit, non-silent statement of the real finding: base `lsp318.ttl`
/// currently declares zero `ADMITTED` methods. All 45 `lsp:Request` + 12
/// `lsp:Notification` individuals (57 total) are `law:CANDIDATE`. This is not
/// a test bug -- it is the honest state of the base ontology as of
/// 2026-08-04 (CP5, not yet run, is the checkpoint that flips specific
/// methods to ADMITTED with a real sync receipt as evidence). Per-example
/// domain overrides (e.g. `examples/ggen-lsp/schema/domain.ttl`) are a
/// separate file and separate ontology graph from this one; they are out of
/// scope for this checkpoint, which targets `ontology/lsp318.ttl` itself.
#[test]
fn admitted_set_is_currently_empty_this_is_a_real_finding() {
    let store = store_from_ttl(LSP318_TTL);
    let admitted = select_admitted_method_names(&store);
    assert!(
        admitted.is_empty(),
        "expected base lsp318.ttl to currently have zero ADMITTED methods; \
         found {:?} -- if this now fails, CP5 (or some other change) has \
         flipped a method to ADMITTED and this test's finding-statement is \
         stale and should be updated, not silently deleted",
        admitted
    );
}

/// SPARQL-derived second proof, independently shaped from the two queries
/// above (a `SELECT`-all + `rdfs:label` join, not a `law:ADMITTED`-only
/// filter): re-derives the full method/status pairing and cross-checks the
/// total individual count (57) plus the count-by-status-label breakdown
/// against a hand count taken directly from `grep -c` over the real file.
#[test]
fn sparql_derived_status_breakdown_matches_hand_counted_grep_totals() {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, LSP318_TTL.as_bytes())
        .expect("lsp318.ttl must parse");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, LAW_AXES_TTL.as_bytes())
        .expect("law-axes.ttl must parse");
    let pairs = select_all_method_status_pairs(&store);
    assert_eq!(
        pairs.len(),
        57,
        "lsp318.ttl declares 45 lsp:Request + 12 lsp:Notification = 57 method \
         individuals (hand-counted via `grep -c 'a lsp:Request'` = 45 and \
         `grep -c 'a lsp:Notification'` = 12 against the real file)"
    );

    let candidate_count = pairs.iter().filter(|(_, s)| s == "CANDIDATE").count();
    let admitted_count = pairs.iter().filter(|(_, s)| s == "ADMITTED").count();
    let refused_count = pairs.iter().filter(|(_, s)| s == "REFUSED").count();
    let unknown_count = pairs.iter().filter(|(_, s)| s == "UNKNOWN").count();

    assert_eq!(
        (candidate_count, admitted_count, refused_count, unknown_count),
        (57, 0, 0, 0),
        "hand count via `grep -o 'law:status *law:[A-Z]*' ontology/lsp318.ttl | sort | \
         uniq -c` against the real file gives exactly 57 CANDIDATE and zero of any \
         other status; SPARQL-derived breakdown disagreed"
    );
}

/// Drift-injection-and-revert: `lsp:exit` is a real individual in
/// `lsp318.ttl` (`law:status law:CANDIDATE`) whose method name `"exit"` is
/// genuinely absent from `EXPECTED_REGISTERED` -- confirmed by grep: `exit`
/// is not handled by any `#[rpc(name = "exit")]` in `src/language_server.rs`
/// (it is a transport-layer message handled below the `LanguageServer`
/// trait, per that trait's own module doc in `src/coverage/lsp_coverage.rs`
/// listing it among the 22 correctly-excluded methods). Deliberately flip
/// its status to `ADMITTED` in an in-memory copy of the TTL and confirm
/// `every_admitted_method_is_actually_registered`'s check logic now refuses
/// it -- proving the check is a real detector, not a vacuously-passing
/// no-op. The real on-disk file (`include_str!`'d into `LSP318_TTL`) is
/// never mutated; only a local `String` copy is.
#[test]
fn admitted_check_refuses_a_deliberately_injected_admitted_but_unregistered_method() {
    assert!(
        !EXPECTED_REGISTERED.contains(&"exit"),
        "this drift-injection test requires 'exit' to be genuinely absent from \
         EXPECTED_REGISTERED -- if this now fails, 'exit' became registered and a \
         different unregistered method must be chosen as the injection target"
    );

    let corrupted = LSP318_TTL.replacen(
        "lsp:exit a lsp:Notification ;\n    lsp:methodName \"exit\" ;\n    lsp:snakeName  \"exit\" ;\n    lsp:paramType  \"()\" ;\n    law:status     law:CANDIDATE ;\n    lsp:expectsBinding \"no-params\" ;\n    lsp:producesShape  \"void\" .",
        "lsp:exit a lsp:Notification ;\n    lsp:methodName \"exit\" ;\n    lsp:snakeName  \"exit\" ;\n    lsp:paramType  \"()\" ;\n    law:status     law:ADMITTED ;\n    lsp:expectsBinding \"no-params\" ;\n    lsp:producesShape  \"void\" .",
        1,
    );
    assert_ne!(
        corrupted, LSP318_TTL,
        "the replacen target block must actually exist verbatim in lsp318.ttl for \
         this injection to be real"
    );

    let corrupted_store = store_from_ttl(&corrupted);
    let admitted = select_admitted_method_names(&corrupted_store);
    assert!(
        admitted.contains("exit"),
        "in-memory-corrupted TTL must now declare 'exit' as ADMITTED"
    );
    let registered: BTreeSet<&str> = EXPECTED_REGISTERED.iter().copied().collect();
    let unregistered_but_admitted: Vec<&String> =
        admitted.iter().filter(|m| !registered.contains(m.as_str())).collect();
    assert!(
        !unregistered_but_admitted.is_empty() && unregistered_but_admitted.contains(&&"exit".to_string()),
        "the admitted-but-unregistered check must catch the deliberately injected \
         'exit' -> ADMITTED drift; found unregistered_but_admitted = {:?}",
        unregistered_but_admitted
    );

    // Revert is implicit: `corrupted` was a local String, LSP318_TTL (the real
    // file, read via include_str!) was never touched. Confirm it still passes
    // the real check cleanly.
    let real_store = store_from_ttl(LSP318_TTL);
    let real_admitted = select_admitted_method_names(&real_store);
    assert!(
        real_admitted.is_empty(),
        "the real on-disk lsp318.ttl must still have zero ADMITTED methods after \
         the in-memory-only drift injection above"
    );
}
