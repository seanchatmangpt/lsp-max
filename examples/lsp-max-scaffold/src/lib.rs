//! # lsp-max-scaffold
//!
//! Reference scaffold for new LSP projects in the lsp-max ecosystem.
//! Demonstrates the five-layer law-state model instead of hexagonal architecture.
//!
//! ## Headline: Replay-Verifiable Diagnostics (RVD)
//!
//! A conventional LSP emits diagnostics as unprovable assertions. This scaffold
//! makes every diagnostic carry a **proof**: a witness (the minimal reproducing
//! input) plus a receipt (digests linked into a hash chain). An independent
//! verifier replays the witness and checks the arithmetic — without trusting or
//! running the original server. Forged or tampered diagnostics fail replay and
//! are `REFUSED`. See [`verifiable`] and `docs/RVD.md`.
//!
//! ## Five-Layer Model
//!
//! ```text
//! Layer 1 — Actuation Grammar    src/nouns/         clap-noun-verb CLI
//! Layer 2 — Local LSP State      src/server.rs      LanguageServer impl (read-only)
//! Layer 3 — Law-State Runtime    src/law.rs         ConformanceVector, tri-state
//!           Proof surface        src/verifiable.rs   receipts, witnesses, hash chain
//!           Analyzer             src/analyzer.rs     pure, replayable detection
//!           Diagnostics          src/diagnostics.rs  ScaffoldDiagnostic, bounded codes
//! Layers 4/5                     lsp-max-runtime     AutonomicMesh, mesh routing
//! ```
//!
//! ## Key invariants enforced here
//!
//! - `UNKNOWN` axes never collapse to `ADMITTED` without receipt evidence
//! - The LSP surface is read-only: no file writes, no subprocess execution
//! - All status labels use the bounded vocabulary (never victory language)
//! - Analyzers are pure: replay on the witness reproduces the finding exactly
//!
//! See `AGENTS.md` for the full scaffold constitution.

/// Layer 1 — actuation grammar: noun/verb CLI surface.
pub mod nouns;

/// Layer 2 — local LSP state surface: LanguageServer implementation.
pub mod server;

/// Layer 3 — law-state runtime: ConformanceVector, tri-state law axes.
pub mod law;

/// Replay-verifiable diagnostics: witnesses, receipts, and hash-chain proofs.
pub mod verifiable;

/// Pure, deterministic source analyzers (the replay target).
pub mod analyzer;

/// Law-surfaced diagnostic codes emitted by this server.
pub mod diagnostics;
