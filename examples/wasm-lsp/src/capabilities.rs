//! ServerCapabilities — wasm-lsp CANDIDATE surface
//! Source: schema/domain.ttl
//! Run `lsp-max ggen sync` once gen.toml rules stabilise to regenerate.

use lsp_max::lsp_types_max::*;
use serde_json::json;

/// Typed `ServerCapabilities` used by the LSP lifecycle.
///
/// Only CANDIDATE capabilities are declared; REFUSED entries are omitted.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // CANDIDATE: hover — law:CANDIDATE in domain.ttl; receipt OPEN
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        // CANDIDATE: completion — receipt OPEN
        completion_provider: Some(CompletionOptions {
            ..Default::default()
        }),
        // REFUSED: rename — law:REFUSED; wasm-lsp is read-only
        // rename_provider: omitted
        ..Default::default()
    }
}

/// Raw JSON capability advertisement for inspection by agents and CI gates.
///
/// Mirrors `server_capabilities()` in JSON form; useful for conformance
/// scoring without spinning up a full LSP session.
pub fn server_capabilities_json() -> serde_json::Value {
    json!({
        // CANDIDATE: hover — receipt OPEN
        "hoverProvider": true,
        // CANDIDATE: completion — receipt OPEN
        "completionProvider": {},
        // REFUSED: rename omitted — law:REFUSED in domain.ttl
        // REFUSED: formatting omitted — read-only surface
        "wasmTransportStatus": "CANDIDATE"
    })
}
