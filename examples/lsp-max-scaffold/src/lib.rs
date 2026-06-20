/// Layer 1 — actuation grammar: noun/verb CLI surface.
pub mod nouns;

/// Layer 2 — local LSP state surface: LanguageServer implementation.
pub mod server;

/// Layer 3 — law-state runtime: ConformanceVector, tri-state law axes.
pub mod law;

/// Law-surfaced diagnostic codes emitted by this server.
pub mod diagnostics;
