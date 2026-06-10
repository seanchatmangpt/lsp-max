# lsp-max-specgen

Generator target for `lsp-max` protocol types.

Input: official LSP 3.18 meta-model JSON:

```text
https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.18/metaModel/metaModel.json
```

Current official spec page marks LSP `3.18` as current, and the meta-model declares `3.18.0`.

## Usage

```bash
curl -fsSL \
  https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.18/metaModel/metaModel.json \
  -o metaModel.json

cargo run -- \
  --input metaModel.json \
  --output generated/lsp_3_18.rs \
  --include-proposed
```

## Design law

This crate does not treat the LSP spec as prose. It treats the official meta-model as the law source and lowers it into Rust protocol shapes.

The first generated layer is deliberately conservative:

- base types map to Rust primitives / `String` / `serde_json::Value`
- structures map to `#[derive(Serialize, Deserialize)]` structs
- open enumerations map to transparent newtypes with associated value constants
- closed enumerations map to serde enums
- complex `or` / `and` types are preserved in comments and currently lower to `serde_json::Value` unless a stable named form is generated later

For `lsp-max`, this generated protocol layer is not the law-state engine. It is the transport vocabulary that the law-state engine projects through.
