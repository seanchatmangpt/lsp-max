//! # Why ConformanceVector Has an Unknown Axis
//!
//! This example is **Explanation** (Diataxis): it explains the rationale behind
//! the three-valued `ConformanceVector` (Admitted / Refused / Unknown) and why
//! collapsing Unknown into either Admitted or Refused is a defect.
//!
//! ## The naive two-valued model and its failure mode
//!
//! A boolean `is_admitted: bool` seems sufficient:
//! - `true`  → the server claims conformance
//! - `false` → the server refuses
//!
//! The problem: a freshly started server, a server mid-initialization, a server
//! whose conformance check timed out, and a server that has never been asked all
//! return `false`. A consumer cannot distinguish "refused because the law was
//! checked and failed" from "refused because we have no evidence either way."
//!
//! In a multi-agent admission pipeline this ambiguity is fatal: an agent that
//! sees `false` may retry, escalate, or open a gate it should not open — all
//! because it cannot tell refusal from ignorance.
//!
//! ## The three-valued model
//!
//! ```
//! pub struct ConformanceVector {
//!     pub admitted: Vec<LawAxis>,   // checked and passed
//!     pub refused:  Vec<LawAxis>,   // checked and failed
//!     pub unknown:  Vec<LawAxis>,   // not yet checked / evidence absent
//! }
//! ```
//!
//! Each law axis (e.g. `OcelFitness`, `ReceiptIntegrity`, `VersionLaw`) can be
//! in exactly one set. The invariant is:
//!
//! ```text
//! admitted ∩ refused  = ∅
//! admitted ∩ unknown  = ∅
//! refused  ∩ unknown  = ∅
//! ```
//!
//! A vector is **fully resolved** when `unknown` is empty. Until then, any
//! downstream gate that requires a resolved vector must block or escalate —
//! it must not treat unknown axes as implicitly admitted.
//!
//! ## Why Unknown must not collapse into Admitted
//!
//! Collapsing Unknown → Admitted is the "optimistic default" mistake. It means:
//! - A server that has never run its OCEL conformance check appears admitted
//! - CI passes without evidence
//! - The receipt chain is bypassed silently
//!
//! This is the exact failure mode the anti-llm-cheat-lsp canary watches for.
//!
//! ## Why Unknown must not collapse into Refused
//!
//! Collapsing Unknown → Refused is the "pessimistic default" mistake. It means:
//! - A server that starts up before evidence is available is permanently refused
//! - Incremental admission (run checks in background, update vector as results
//!   arrive) is impossible
//! - False negatives block legitimate agents
//!
//! ## The correct consumer contract
//!
//! A gate that reads a `ConformanceVector` must:
//! 1. Check that the relevant axes are in `admitted` (not merely absent from `refused`)
//! 2. If any required axis is in `unknown`, block or schedule a re-check
//! 3. Never infer admission from the absence of a refused entry
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │ Gate decision logic                                 │
//! │                                                     │
//! │ for each required_axis:                             │
//! │   if axis in admitted  → continue                   │
//! │   if axis in refused   → REFUSE immediately         │
//! │   if axis in unknown   → BLOCK (do not admit)       │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## How vectors are populated in lsp-max
//!
//! - At startup: all axes start in `unknown`
//! - After `initialize` completes: capability axes move to `admitted` or `refused`
//! - After OCEL conformance check: `OcelFitness` moves out of `unknown`
//! - After receipt verification: `ReceiptIntegrity` moves out of `unknown`
//!
//! The `AutonomicMesh` in `lsp-max-runtime` drives these transitions via hooks.
//! A `ConformanceDeltaEntry` is appended to the delta log each time an axis moves,
//! enabling `max/conformanceDelta` polling by agents.

// The explanation above is the *claim*. The code below is the *witness*: it
// constructs real `ConformanceVector`s and asserts the three-valued contract,
// so this example FAILS TO RUN (panics, non-zero exit) if Unknown ever collapses
// into Admitted or Refused. Run it:  cargo run --example conformance_vector_explained
//
// Type:      lsp-max-protocol/src/conformance.rs
// Gate use:  src/gate.rs
// Delta log: src/lib.rs (conformance_delta_log field)

use lsp_max::max_protocol::conformance::LawAxisRegistry;
use lsp_max::max_protocol::{ConformanceVector, LawAxis};

/// Build a vector from the three axis sets, syncing the bitmask index — the
/// documented construction idiom (see `sync_bits_from_vecs`).
fn vector(
    admitted: Vec<LawAxis>,
    refused: Vec<LawAxis>,
    unknown: Vec<LawAxis>,
    strict: bool,
) -> ConformanceVector {
    let mut cv = ConformanceVector {
        admitted,
        refused,
        unknown,
        strict_mode: strict,
        ..Default::default()
    };
    cv.sync_bits_from_vecs();
    cv
}

fn main() {
    // [1] Fully resolved, all admitted ⇒ release is admitted.
    let all_ok = vector(
        vec![LawAxis::Protocol, LawAxis::Receipt],
        vec![],
        vec![],
        true,
    );
    assert!(all_ok.all_admitted(), "no refused/unknown ⇒ all_admitted");
    assert!(all_ok.admits_release(), "fully admitted ⇒ release admitted");

    // [2] THE LOAD-BEARING LAW: an unknown axis is NOT admitted, and under strict
    //     mode it BLOCKS release. Unknown does not optimistically collapse to Admitted.
    let with_unknown = vector(
        vec![LawAxis::Protocol],
        vec![],
        vec![LawAxis::Receipt],
        true,
    );
    assert!(
        !with_unknown.all_admitted(),
        "unknown present ⇒ NOT all_admitted (no optimistic collapse)"
    );
    assert!(
        !with_unknown.admits_release(),
        "strict mode: an unknown axis BLOCKS release"
    );

    // [3] Same vector, non-strict: unknown is tolerated for release but STILL not
    //     counted as admitted — toleration is not admission.
    let with_unknown_lax = vector(
        vec![LawAxis::Protocol],
        vec![],
        vec![LawAxis::Receipt],
        false,
    );
    assert!(
        with_unknown_lax.admits_release(),
        "non-strict: unknown tolerated for release"
    );
    assert!(
        !with_unknown_lax.all_admitted(),
        "non-strict still does NOT count unknown as admitted"
    );

    // [4] A refused axis blocks release in any mode. Refused is distinct from
    //     Unknown — Unknown never collapses to Refused either.
    let with_refused = vector(
        vec![LawAxis::Protocol],
        vec![LawAxis::Security],
        vec![],
        false,
    );
    assert!(
        !with_refused.admits_release(),
        "refused ⇒ release blocked even non-strict"
    );

    // [5] Transition integrity via the bitmask index: moving an axis between sets
    //     clears the prior set, so an axis can never be simultaneously unknown and
    //     admitted — the exact overlap this three-valued type forbids.
    let id = LawAxisRegistry::axis_to_id(&LawAxis::Receipt).expect("Receipt axis has an id");
    let mut cv = ConformanceVector::default();
    cv.set_unknown(id);
    assert!(
        cv.is_unknown_bit(id) && !cv.is_admitted_bit(id),
        "set_unknown ⇒ axis is unknown, not admitted"
    );
    cv.set_admitted(id);
    assert!(
        cv.is_admitted_bit(id) && !cv.is_unknown_bit(id),
        "set_admitted clears the unknown bit — sets stay disjoint"
    );
    cv.assert_bitmask_invariants(); // panics if any two sets overlap

    println!("WITNESS conformance_vector: 5 contract assertions held");
    println!("  [1] all-admitted vector admits release");
    println!("  [2] unknown axis is NOT admitted and BLOCKS release under strict mode");
    println!("  [3] non-strict tolerates unknown for release but never counts it admitted");
    println!("  [4] refused axis blocks release in any mode (distinct from unknown)");
    println!("  [5] set_unknown→set_admitted keeps the three axis sets disjoint");
    println!();
    println!("This example panics (non-zero exit) if Unknown ever collapses into");
    println!("Admitted or Refused — that regression would break the gate contract.");
}
