---
name: rule-pack-server
description: Implement or debug a RulePackServer — the bridge trait for building diagnostic LSP servers in lsp-max. Use when adding a new server or extending an existing one.
tools: [Read, Grep, Edit, Write, Bash]
---

# RulePackServer Implementation

`RulePackServer` (`src/rule_pack_server.rs`) eliminates hand-rolled LSP boilerplate for diagnostic servers.

## Minimal Implementation (TOML-pack server)

```rust
use lsp_max::{Client, ClassifiedFindings, RulePackServer, ValidatedRulePackSet, WorkspaceIndex};
use lsp_max_ast::AutoLspAdapter;

struct MyServer {
    client: Client,
    rule_packs: ValidatedRulePackSet,
    workspace_index: WorkspaceIndex,
    ast_adapter: crate::ast_adapter::MyAstAdapter,
}

impl RulePackServer for MyServer {
    fn rule_packs(&self) -> &ValidatedRulePackSet { &self.rule_packs }
    fn grammar(&self) -> tree_sitter::Language { tree_sitter_rust::LANGUAGE.into() }
    fn server_name(&self) -> &'static str { "my-server" }
    fn client(&self) -> &Client { &self.client }
    fn adapter(&self) -> &AutoLspAdapter { self.ast_adapter.inner() }
    fn workspace_index(&self) -> Option<&WorkspaceIndex> { Some(&self.workspace_index) }
}
```

## Engine-Bridge Pattern (when you have an existing scanner, e.g. AhoCorasick)

Override `scan_uri_classified` to bridge your engine into `ClassifiedFindings`:

```rust
fn scan_uri_classified(&self, uri: &DocumentUri, _content: &str) -> ClassifiedFindings {
    let root_dir = self.root_dir();
    let obs = engine::scan_directory(&root_dir);
    let raw = engine::evaluate_diagnostics(&obs);
    let norm_uri = uri.to_string().replace('\\', "/");
    let findings: Vec<Finding> = raw.into_iter()
        .filter(|d| norm_uri.ends_with(&d.file_path.replace('\\', "/")))
        .map(|d| {
            let lsp_diag = d.to_lsp();
            let law_axis = LawAxis::Custom(d.category.clone());
            let max_diag = MaxDiagnostic {
                lsp: lsp_diag.clone(),
                diagnostic_id: d.code.clone(),
                law_id: d.category.clone(),
                law_axis,
                violated_invariant: d.forbidden_implication.clone(),
                ..MaxDiagnostic::default()
            };
            (max_diag, lsp_diag)
        }).collect();
    (findings, vec![]) // (sync, background)
}
```

## Key Types

| Type | Re-export from | Description |
|------|----------------|-------------|
| `ClassifiedFindings` | `lsp_max` | `(Vec<Finding>, Vec<Finding>)` — sync, background |
| `Finding` | `lsp_max` | `(MaxDiagnostic, Diagnostic)` — one finding |
| `ValidatedRulePackSet` | `lsp_max` | Monoid newtype; `::empty()` for engine-bridge |
| `WorkspaceIndex` | `lsp_max` | `Arc<DashMap<String, IndexedDoc>>` |
| `LawAxis` | `lsp_max::max_protocol` | Use `Custom(String)` for domain categories |

## LawAxis Variants
`Protocol`, `Type`, `Fixture`, `Documentation`, `Release`, `Hook`, `Repair`, `Receipt`, `Security`, `Autopoiesis`, `Domain`, `Custom(String)`.

**Never invent new enum variants.** Use `Custom("your-category")` for domain-specific axes.

## did_open / did_change wiring

When adopting `RulePackServer`, delegate `did_open` and `did_change` to the trait:
```rust
async fn did_open(&self, params: DidOpenTextDocumentParams) {
    <Self as RulePackServer>::handle_did_open(self, params).await;
    self.fire_refreshes(); // virtual doc refresh, NOT re-publish
}
```

**Do not call `run_scan_and_publish` after `handle_did_open`** — the trait already calls `publish_findings_classified`. Double-publish emits duplicate diagnostics to the editor.

## did_close wiring

```rust
async fn did_close(&self, params: DidCloseTextDocumentParams) {
    <Self as RulePackServer>::handle_did_close(self, params.clone());
    self.run_scan_and_publish(&params.text_document.uri).await;
}
```

## Reference Implementation

`crates/anti-llm-cheat-lsp/src/server.rs` — engine-bridge pattern with AhoCorasick + virtual docs.
`examples/pattern-lsp/src/server.rs` — TOML-pack pattern with WorkspaceIndex.
