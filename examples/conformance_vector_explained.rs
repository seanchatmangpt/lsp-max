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
//! This is the exact failure mode the anti-llm-lsp canary watches for.
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

// This file is intentionally a documentation-only example.
//
// To see the ConformanceVector type: lsp-max-protocol/src/lib.rs
// To see gate logic using it:       src/gate.rs
// To see delta log:                 src/lib.rs (conformance_delta_log field)

fn main() {
    println!("ConformanceVector explanation: see module-level doc comment above.");
    println!("For the type definition: lsp-max-protocol/src/lib.rs");
}
