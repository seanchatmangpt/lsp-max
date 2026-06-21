//! # lsp-max-scaffold
//!
//! Reference scaffold for new LSP projects in the lsp-max ecosystem.
//! Demonstrates the five-layer law-state model instead of hexagonal architecture.
//!
//! ## Five-Layer Model
//!
//! ```text
//! Layer 1 — Actuation Grammar    src/nouns/         clap-noun-verb CLI
//! Layer 2 — Local LSP State      src/server.rs      LanguageServer impl (read-only)
//! Layer 3 — Law-State Runtime    src/law.rs         ConformanceVector, tri-state
//!           Diagnostics          src/diagnostics.rs  ScaffoldDiagnostic, bounded codes
//! Layers 4/5                     lsp-max-runtime     AutonomicMesh, mesh routing
//! ```
//!
//! ## Key invariants enforced here
//!
//! - `UNKNOWN` axes never collapse to `ADMITTED` without receipt evidence
//! - The LSP surface is read-only: no file writes, no subprocess execution
//! - All status labels use the bounded vocabulary (never victory language)
//! - The ANDON gate is checked on every document event before emitting diagnostics
//!
//! See `AGENTS.md` for the full scaffold constitution.

/// Layer 1 — actuation grammar: noun/verb CLI surface.
pub mod nouns;

/// Layer 2 — local LSP state surface: LanguageServer implementation.
pub mod server;

/// Layer 3 — law-state runtime: ConformanceVector, tri-state law axes.
pub mod law;

/// Law-surfaced diagnostic codes emitted by this server.
pub mod diagnostics;
