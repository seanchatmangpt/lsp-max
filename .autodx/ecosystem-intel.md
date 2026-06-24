# Ecosystem AGI Manifest
Generated automatically by Justfile.

## Architectural Mandate
- **wasm4pm-compat**: The sole, axiomatic baseline type authority.
- **wasm4pm**: The execution engine.
- **lsp-max**: The semantic intelligence layer.

## Forbidden Concepts
- No deprecation paths.
- No legacy terminology.
- No intermediary type crates (e.g., wasm4pm-types, ocel-core).

## Crate Topography
lsp-max v26.6.9 (/Users/sac/lsp-max)
├── async-trait v0.1.89 (proc-macro)
├── auto_impl v1.3.0 (proc-macro)
├── bytes v1.11.1
├── dashmap v6.2.1
├── futures v0.3.31
├── httparse v1.10.1
├── libc v0.2.186
├── lsp-max-agent v26.6.9 (/Users/sac/lsp-max/lsp-max-agent)
├── lsp-max-ast v26.6.9 (/Users/sac/lsp-max/crates/lsp-max-adapters/lsp-max-ast)
├── lsp-max-base v26.6.9 (/Users/sac/lsp-max/crates/lsp-max-base)
├── lsp-max-lsif v26.6.9 (/Users/sac/lsp-max/crates/lsp-max-lsif)
├── lsp-max-macros v26.6.9 (proc-macro) (/Users/sac/lsp-max/lsp-max-macros)
├── lsp-max-protocol v26.6.9 (/Users/sac/lsp-max/lsp-max-protocol)
├── lsp-max-runtime v26.6.9 (/Users/sac/lsp-max/lsp-max-runtime)
├── lsp-types-max v26.6.24 (/Users/sac/lsp-types-max)
├── memchr v2.8.1
├── parking_lot v0.12.5
├── regex v1.12.3
├── rustc-hash v2.1.1
├── serde v1.0.228
├── serde_json v1.0.149
├── tokio v1.47.5
├── tokio-util v0.7.16
├── tower v0.4.13
├── tracing v0.1.44
├── tree-sitter v0.26.9
│   [build-dependencies]
├── url v2.5.8
├── wasm4pm-compat v26.6.24 (/Users/sac/wasm4pm-compat)
└── windows-sys v0.52.0
[dev-dependencies]
├── async-tungstenite v0.29.1
├── blake3 v1.8.5
│   [build-dependencies]
├── ed25519-dalek v2.1.1
├── lsif-rust v26.6.9 (/Users/sac/lsp-max/crates/lsif-rust)
│   [build-dependencies]
├── lsif-typescript v26.6.9 (/Users/sac/lsp-max/crates/lsif-typescript)
│   [build-dependencies]
├── lsp-max-playground v26.6.9 (/Users/sac/lsp-max/crates/playground)
├── oxigraph v0.5.8
├── ropey v1.6.1
├── tempfile v3.27.0
├── tokio v1.47.5 (*)
├── tokio-util v0.7.16 (*)
├── tracing-subscriber v0.3.23
├── tree-sitter-rust v0.23.3
│   [build-dependencies]
├── uuid v1.23.2
├── walkdir v2.5.0
├── wasm4pm-compat v26.6.24 (/Users/sac/wasm4pm-compat) (*)
└── ws_stream_tungstenite v0.15.0
    [build-dependencies]
