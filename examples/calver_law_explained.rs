//! # Why lsp-max Uses CalVer Instead of SemVer
//!
//! This example is **Explanation** (Diataxis): it explains the rationale behind
//! the CalVer (`YY.M.D`) versioning scheme and why SemVer would be wrong here.
//!
//! ## What CalVer means in this project
//!
//! lsp-max versions look like `26.6.9`:
//! - `26` = year 2026
//! - `6`  = month June
//! - `9`  = day 9
//!
//! A version uniquely identifies a release date, not a compatibility contract.
//!
//! ## Why SemVer does not fit
//!
//! SemVer (`MAJOR.MINOR.PATCH`) communicates API compatibility:
//! - PATCH: bug fix, no API change
//! - MINOR: new backwards-compatible feature
//! - MAJOR: breaking change
//!
//! lsp-max's reality makes these distinctions unreliable:
//!
//! **The LSP spec itself is not stable.** LSP 3.18 adds `proposed` features that
//! may change between spec revisions. A version bump that adds a proposed feature
//! is simultaneously "new feature" and "potentially breaking" depending on whether
//! the downstream client uses the proposed flag.
//!
//! **`max/*` protocol methods evolve with the law model.** `ConformanceVector`,
//! receipt schemas, and gate contracts change as the admission model matures. These
//! are not additive features — they are living law specifications. Calling a change
//! "MINOR" when it changes a gate's admission semantics is misleading.
//!
//! **Agents and CI are the primary clients.** Agents are updated with the server;
//! they do not consume a stable published API across versions. The "compatibility
//! signal" SemVer provides has no consumer.
//!
//! ## What CalVer provides instead
//!
//! A CalVer version is a **timestamp claim**: "this artifact was produced from the
//! source state on this date." Combined with receipts and OCEL logs, it is part of
//! the audit trail:
//!
//! ```text
//! release artifact  →  version = 26.6.9
//! receipt digest    →  BLAKE3 of artifact bytes
//! OCEL event log    →  VersionLaw axis admitted at 2026-06-09
//! ```
//!
//! This makes "when was this released?" answerable from the version string alone,
//! without querying a changelog or git log.
//!
//! ## The version-law diagnostic family
//!
//! `anti-llm-cheat-lsp` enforces CalVer through the `ANTI-LLM-VERSION-*` diagnostic
//! family. Any `Cargo.toml` containing `version = "1.0.0"` or `version = "0.x.y"`
//! in a published crate is a `ANTI-LLM-VERSION-001` violation. The canary watches
//! for these because LLMs and scaffolding tools default to SemVer placeholders.
//!
//! ## How to read sibling repo version constraints
//!
//! Because sibling repos (`lsp-types-max`, `wasm4pm-compat`) are path dependencies
//! with version constraints, the version constraint must match the *sibling's actual
//! CalVer tag* — not the workspace version:
//!
//! ```toml
//! # WRONG: assumes sibling bumps in lockstep
//! lsp-types-max = { path = "../lsp-types-max", version = "26.6.9" }
//!
//! # CORRECT: tracks the sibling's last-known tag
//! lsp-types-max = { path = "../lsp-types-max", version = "26.6.8" }
//! ```
//!
//! Blanket version-bump scripts that replace all occurrences of the old version
//! string will break sibling constraints if the sibling has not yet been released
//! with the new version. Always verify sibling versions independently.
//!
//! ## Summary
//!
//! | Property | SemVer | CalVer (lsp-max) |
//! |----------|--------|-----------------|
//! | Communicates | API compatibility | Release date |
//! | Meaningful for agents? | No | Yes (audit trail) |
//! | Stable API contract | Implied | Not claimed |
//! | LLM-forgeable default | Yes (`1.0.0`) | No (date-anchored) |
//! | Enforced by tooling | No | Yes (`ANTI-LLM-VERSION-*`) |

// This file is intentionally a documentation-only example.
//
// To see the version in use:         Cargo.toml (workspace.package.version)
// To see the version-law diagnostic: examples/anti-llm-cheat-lsp/src/
// To see how sibling versions work:  Cargo.toml ([patch.crates-io] section)

fn main() {
    println!("CalVer law explanation: see module-level doc comment above.");
    println!("Version enforcement: examples/anti-llm-cheat-lsp/src/");
}
