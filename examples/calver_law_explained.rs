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

// The explanation above is the *claim*. The code below is the *witness*: it
// validates this crate's ACTUAL version (env!("CARGO_PKG_VERSION")) against the
// CalVer YY.M.D law and rejects SemVer-shaped strings, so this example FAILS
// (panics, non-zero exit) the moment the workspace version stops being lawful
// CalVer. Run it:  cargo run --example calver_law_explained
//
// Version source:        Cargo.toml (workspace.package.version)
// Law enforcement (prod): examples/anti-llm-cheat-lsp/src/rules/version.rs
//   (the ANTI-LLM-VERSION-* diagnostic family enforces this on Cargo.toml)

/// Is `s` a lawful CalVer `YY.M.D`? Three numeric components, a plausible
/// two-digit-or-more year, month 1–12, day 1–31. This is the same shape the
/// `ANTI-LLM-VERSION-*` rule enforces on the workspace manifest.
fn is_calver(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        parts[0].parse::<u32>(),
        parts[1].parse::<u32>(),
        parts[2].parse::<u32>(),
    ) else {
        return false;
    };
    year >= 24 && (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn main() {
    // [1] The load-bearing assertion: THIS crate's real version is lawful CalVer.
    //     If the workspace is ever bumped to a SemVer or malformed version, this
    //     example stops running — the version law has a live witness, not just prose.
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        is_calver(version),
        "workspace version {version:?} must be lawful CalVer YY.M.D"
    );

    // [2] Positive cases: real release-date-shaped versions are accepted.
    for ok in ["26.6.9", "24.1.1", "26.12.31"] {
        assert!(is_calver(ok), "{ok} should be valid CalVer");
    }

    // [3] Rejection — the edge cases the law is *for*. SemVer-shaped and malformed
    //     versions must be refused, or "CalVer enforcement" would be vacuous.
    for bad in [
        "1.2.3",      // year 1 — not a plausible 20YY year
        "26.13.1",    // month 13 — impossible
        "26.6.32",    // day 32 — impossible
        "26.6.0",     // day 0 — impossible
        "26.6",       // wrong arity (SemVer-less patch)
        "v26.6.9",    // non-numeric prefix
        "26.6.9-rc1", // SemVer pre-release suffix
    ] {
        assert!(!is_calver(bad), "{bad} must be rejected as non-CalVer");
    }

    println!("WITNESS calver_law: version law holds for this crate");
    println!("  actual CARGO_PKG_VERSION = {version} (valid YY.M.D)");
    println!("  [1] this crate's real version is lawful CalVer");
    println!("  [2] release-date-shaped versions accepted (26.6.9, 24.1.1, 26.12.31)");
    println!("  [3] SemVer/malformed rejected (1.2.3, 26.13.1, 26.6.32, 26.6, v.., -rc1)");
    println!();
    println!("This example panics if the workspace version stops being lawful CalVer —");
    println!("the same law examples/anti-llm-cheat-lsp/src/rules/version.rs enforces.");
}
