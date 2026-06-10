# lsp-max

[![Build Status][build-badge]][build-url]
[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]

[build-badge]: https://github.com/seanchatmangpt/lsp-max/workflows/rust/badge.svg
[build-url]: https://github.com/seanchatmangpt/lsp-max/actions
[crates-badge]: https://img.shields.io/crates/v/lsp-max.svg
[crates-url]: https://crates.io/crates/lsp-max
[docs-badge]: https://docs.rs/lsp-max/badge.svg
[docs-url]: https://docs.rs/lsp-max

Law-state LSP runtime — maximum [LSP 3.18] coverage, process-mining conformance,
and receipt-chain admission. Primary clients are agents, CI, and release gates;
the editor is one client among many.

[LSP 3.18]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/

## What this is

`lsp-max` is a fork of `tower-lsp` that diverged into a "law-state runtime
projected through LSP." It adds:

- **`max/*` protocol surface** — snapshots, conformance vectors, receipts, repair
  plans, and gates beyond standard LSP.
- **Receipt-chain admission** — every capability claim requires a BLAKE3-hashed
  receipt; tests without receipts are not admitted.
- **Process-mining conformance** — OCEL event logs derived from OTel traces are
  checked against declared process models via `wasm4pm`.
- **`ConformanceVector`** — `admitted`/`refused`/`unknown` axes; unknown never
  collapses into admitted or refused.

## Published crates

| Crate | Description |
|-------|-------------|
| `lsp-max` | LSP server framework: `LanguageServer` trait, `LspService`, `Server`, law-state surface |
| `lsp-max-macros` | Proc macros (`#[lsp_max::async_trait]`) |
| `lsp-max-cli` | Actuation grammar: noun/verb CLI built on `clap-noun-verb` |
| `lsp-max-client` | LSP client framework for driving servers in tests and agents |

All other workspace crates are internal implementation details (`publish = false`).

## Quick start

```toml
[dependencies]
lsp-max = "26.6"
```

```rust
use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types_max::*;
use lsp_max::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[lsp_max::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

## Versioning

`lsp-max` uses **CalVer** (`YY.M.D`). Version `26.6.9` = 2026-06-09. There are
no SemVer guarantees — version-law violations are a diagnostic family
(`ANTI-LLM-VERSION-*`).

## Sibling repo dependencies

The workspace requires sibling checkouts at:

- `../lsp-types-max` — LSP type authority (with LSP 3.18 `proposed` features)
- `../wasm4pm-compat` — baseline process-mining type authority
- `../wasm4pm` — process-mining execution engine

## Examples

Domain-specific LSP servers are in `examples/`:

| Example | Description |
|---------|-------------|
| `anti-llm-lsp` | Canary: detects forbidden identifiers, fake receipts, victory language |
| `clap-noun-verb-lsp` | Noun/verb CLI surface demo |
| `pattern-lsp` | Pattern detection LSP |
| `wasm4pm-lsp` | Process-mining LSP over wasm4pm |
| `axum-lsp`, `bevy-lsp`, `tex-lsp` | Framework integration demos |

## Proposed features

Enable LSP 3.18 proposed features:

```toml
[dependencies]
lsp-max = { version = "26.6", features = ["proposed"] }
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your
option.
