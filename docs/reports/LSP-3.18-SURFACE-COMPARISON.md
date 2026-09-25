# LSP 3.18 Surface Comparison Report

- **Metadata Version:** 3.18.0
- **Date of Analysis:** 2026-06-04

---

## Executive Summary

This report provides a detailed, comprehensive surface area comparison between the official Language Server Protocol (LSP) 3.18.0 specification (the meta-model) and its implementation in `tower-lsp-max`. 

The analysis focuses on mapping out every single request, notification, structure, enumeration, and type alias defined in the LSP 3.18.0 meta-model. It categorizes each element's representation in the generated Rust types, its routing coverage on the server, its integration via client-side helper structures, and its explicit support status.

### Summary of Surface Metrics

Category | Meta-Model Count | Represented in Rust Crate? | Server Routed? | Client Helper? | Intentionally Unsupported?
--- | --- | --- | --- | --- | ---
**Requests** | 69 | 69 (100%) | 54 | 12 | 4 (Progress/Folding/Format)
**Notifications** | 26 | 26 (100%) | 14 (12 trait, 2 direct) | 5 | 7 (Notebook/Tracing/Cancel)
**Structures** | 387 | 387 (100%) | N/A | N/A | N/A
**Enumerations** | 40 | 40 (100%) | N/A | N/A | N/A
**Type Aliases** | 22 | 22 (100%) | N/A | N/A | N/A

---

## The Core Concept: Vocabulary vs. Routing vs. Implementation Coverage

When evaluating protocol compatibility, it is vital to distinguish between different coverage categories:

1. **Vocabulary Coverage (Types):** The compilation-level existence of structures and data structures representing the protocol definitions.
   - *Example:* The structure `InlineCompletionParams` exists in the `tower-lsp-max-protocol` library. This is vocabulary-complete.
2. **Routing Coverage (Router Registration):** The configuration of the JSON-RPC server router to accept and delegate a specific method string rather than returning a generic transport-level error.
   - *Example:* The method `textDocument/inlineCompletion` is declared on the `LanguageServer` trait. When the server starts, the router registers this method.
3. **Implementation Coverage (Behavioral Logic):** The actual code execution block that processes the client request, implements logic, and responds with semantics.
   - *Critique:* A routed method that returns `Error::method_not_found()` represents **Routing Coverage ONLY**, and has **NO Implementation Coverage**. In `tower-lsp-max`, the three new LSP 3.18.0 client-to-server requests (`textDocument/inlineCompletion`, `workspace/textDocumentContent`, and `workspace/textDocumentContent/refresh`) are routed but unimplemented.

---

## 1. Requests

Below is the exhaustive comparison of all 69 requests in the LSP 3.18.0 meta-model.

# | Request Method | Direction | Exposed in Rust? | Server Routed? | Client Helper? | Status / Notes
--- | --- | --- | --- | --- | --- | ---
1 | `textDocument/implementation` | clientToServer | Yes | Yes | No | Fully supported
2 | `textDocument/typeDefinition` | clientToServer | Yes | Yes | No | Fully supported
3 | `workspace/workspaceFolders` | serverToClient | Yes | No | Yes | `Client::workspace_folders`
4 | `workspace/configuration` | serverToClient | Yes | No | Yes | `Client::configuration`
5 | `textDocument/documentColor` | clientToServer | Yes | Yes | No | Fully supported
6 | `textDocument/colorPresentation` | clientToServer | Yes | Yes | No | Fully supported
7 | `textDocument/foldingRange` | clientToServer | Yes | Yes | No | Fully supported
8 | `workspace/foldingRange/refresh` | serverToClient | Yes | No | No | **Unsupported** (missing client helper)
9 | `textDocument/declaration` | clientToServer | Yes | Yes | No | Fully supported
10 | `textDocument/selectionRange` | clientToServer | Yes | Yes | No | Fully supported
11 | `window/workDoneProgress/create` | serverToClient | Yes | No | No | **Unsupported** (marked with TODO in `client.rs`)
12 | `textDocument/prepareCallHierarchy` | clientToServer | Yes | Yes | No | Fully supported
13 | `callHierarchy/incomingCalls` | clientToServer | Yes | Yes | No | Fully supported
14 | `callHierarchy/outgoingCalls` | clientToServer | Yes | Yes | No | Fully supported
15 | `textDocument/semanticTokens/full` | clientToServer | Yes | Yes | No | Fully supported
16 | `textDocument/semanticTokens/full/delta` | clientToServer | Yes | Yes | No | Fully supported
17 | `textDocument/semanticTokens/range` | clientToServer | Yes | Yes | No | Fully supported
18 | `workspace/semanticTokens/refresh` | serverToClient | Yes | No | Yes | `Client::semantic_tokens_refresh`
19 | `window/showDocument` | serverToClient | Yes | No | Yes | `Client::show_document`
20 | `textDocument/linkedEditingRange` | clientToServer | Yes | Yes | No | Fully supported
21 | `workspace/willCreateFiles` | clientToServer | Yes | Yes | No | Fully supported
22 | `workspace/willRenameFiles` | clientToServer | Yes | Yes | No | Fully supported
23 | `workspace/willDeleteFiles` | clientToServer | Yes | Yes | No | Fully supported
24 | `textDocument/moniker` | clientToServer | Yes | Yes | No | Fully supported
25 | `textDocument/prepareTypeHierarchy` | clientToServer | Yes | Yes | No | Fully supported
26 | `typeHierarchy/supertypes` | clientToServer | Yes | Yes | No | Fully supported
27 | `typeHierarchy/subtypes` | clientToServer | Yes | Yes | No | Fully supported
28 | `textDocument/inlineValue` | clientToServer | Yes | Yes | No | Fully supported
29 | `workspace/inlineValue/refresh` | serverToClient | Yes | No | Yes | `Client::inline_value_refresh`
30 | `textDocument/inlayHint` | clientToServer | Yes | Yes | No | Fully supported
31 | `inlayHint/resolve` | clientToServer | Yes | Yes | No | Fully supported
32 | `workspace/inlayHint/refresh` | serverToClient | Yes | No | Yes | `Client::inlay_hint_refresh`
33 | `textDocument/diagnostic` | clientToServer | Yes | Yes | No | Fully supported
34 | `workspace/diagnostic` | clientToServer | Yes | Yes | No | Fully supported
35 | `workspace/diagnostic/refresh` | serverToClient | Yes | No | Yes | `Client::workspace_diagnostic_refresh`
36 | `textDocument/inlineCompletion` | clientToServer | Yes | Yes | No | Routed; stub returns `method_not_found`
37 | `workspace/textDocumentContent` | clientToServer | Yes | Yes | No | Routed; stub returns `method_not_found`
38 | `workspace/textDocumentContent/refresh` | serverToClient | Yes | Yes | No | **Mismatched**: Routed on server but should be client helper
39 | `client/registerCapability` | serverToClient | Yes | No | Yes | `Client::register_capability`
40 | `client/unregisterCapability` | serverToClient | Yes | No | Yes | `Client::unregister_capability`
41 | `initialize` | clientToServer | Yes | Yes | No | Fully supported
42 | `shutdown` | clientToServer | Yes | Yes | No | Fully supported
43 | `window/showMessageRequest` | serverToClient | Yes | No | Yes | `Client::show_message_request`
44 | `textDocument/willSaveWaitUntil` | clientToServer | Yes | Yes | No | Fully supported
45 | `textDocument/completion` | clientToServer | Yes | Yes | No | Fully supported
46 | `completionItem/resolve` | clientToServer | Yes | Yes | No | Fully supported
47 | `textDocument/hover` | clientToServer | Yes | Yes | No | Fully supported
48 | `textDocument/signatureHelp` | clientToServer | Yes | Yes | No | Fully supported
49 | `textDocument/definition` | clientToServer | Yes | Yes | No | Fully supported
50 | `textDocument/references` | clientToServer | Yes | Yes | No | Fully supported
51 | `textDocument/documentHighlight` | clientToServer | Yes | Yes | No | Fully supported
52 | `textDocument/documentSymbol` | clientToServer | Yes | Yes | No | Fully supported
53 | `textDocument/codeAction` | clientToServer | Yes | Yes | No | Fully supported
54 | `codeAction/resolve` | clientToServer | Yes | Yes | No | Fully supported
55 | `workspace/symbol` | clientToServer | Yes | Yes | No | Fully supported
56 | `workspaceSymbol/resolve` | clientToServer | Yes | Yes | No | Fully supported
57 | `textDocument/codeLens` | clientToServer | Yes | Yes | No | Fully supported
58 | `codeLens/resolve` | clientToServer | Yes | Yes | No | Fully supported
59 | `workspace/codeLens/refresh` | serverToClient | Yes | No | Yes | `Client::code_lens_refresh`
60 | `textDocument/documentLink` | clientToServer | Yes | Yes | No | Fully supported
61 | `documentLink/resolve` | clientToServer | Yes | Yes | No | Fully supported
62 | `textDocument/formatting` | clientToServer | Yes | Yes | No | Fully supported
63 | `textDocument/rangeFormatting` | clientToServer | Yes | Yes | No | Fully supported
64 | `textDocument/rangesFormatting` | clientToServer | Yes | No | No | **Unsupported** (new in 3.18.0)
65 | `textDocument/onTypeFormatting` | clientToServer | Yes | Yes | No | Fully supported
66 | `textDocument/rename` | clientToServer | Yes | Yes | No | Fully supported
67 | `textDocument/prepareRename` | clientToServer | Yes | Yes | No | Fully supported
68 | `workspace/executeCommand` | clientToServer | Yes | Yes | No | Fully supported
69 | `workspace/applyEdit` | serverToClient | Yes | No | Yes | `Client::apply_edit`

---

## 2. Notifications

Below is the exhaustive comparison of all 26 notifications in the LSP 3.18.0 meta-model.

# | Notification Method | Direction | Exposed in Rust? | Server Routed? | Client Helper? | Status / Notes
--- | --- | --- | --- | --- | --- | ---
1 | `workspace/didChangeWorkspaceFolders` | clientToServer | Yes | Yes | No | Fully supported
2 | `window/workDoneProgress/cancel` | clientToServer | Yes | No | No | **Unsupported**
3 | `workspace/didCreateFiles` | clientToServer | Yes | Yes | No | Fully supported
4 | `workspace/didRenameFiles` | clientToServer | Yes | Yes | No | Fully supported
5 | `workspace/didDeleteFiles` | clientToServer | Yes | Yes | No | Fully supported
6 | `notebookDocument/didOpen` | clientToServer | Yes | No | No | **Unsupported** (in `FEATURES.md`)
7 | `notebookDocument/didChange` | clientToServer | Yes | No | No | **Unsupported** (in `FEATURES.md`)
8 | `notebookDocument/didSave` | clientToServer | Yes | No | No | **Unsupported** (in `FEATURES.md`)
9 | `notebookDocument/didClose` | clientToServer | Yes | No | No | **Unsupported** (in `FEATURES.md`)
10 | `initialized` | clientToServer | Yes | Yes | No | Fully supported
11 | `exit` | clientToServer | Yes | Yes | No | Intercepted directly by generated macro router
12 | `workspace/didChangeConfiguration` | clientToServer | Yes | Yes | No | Fully supported
13 | `window/showMessage` | serverToClient | Yes | No | Yes | `Client::show_message`
14 | `window/logMessage` | serverToClient | Yes | No | Yes | `Client::log_message`
15 | `telemetry/event` | serverToClient | Yes | No | Yes | `Client::telemetry_event`
16 | `textDocument/didOpen` | clientToServer | Yes | Yes | No | Fully supported
17 | `textDocument/didChange` | clientToServer | Yes | Yes | No | Fully supported
18 | `textDocument/didClose` | clientToServer | Yes | Yes | No | Fully supported
19 | `textDocument/didSave` | clientToServer | Yes | Yes | No | Fully supported
20 | `textDocument/willSave` | clientToServer | Yes | Yes | No | Fully supported
21 | `workspace/didChangeWatchedFiles` | clientToServer | Yes | Yes | No | Fully supported
22 | `textDocument/publishDiagnostics` | serverToClient | Yes | No | Yes | `Client::publish_diagnostics`
23 | `$/setTrace` | clientToServer | Yes | No | No | **Unsupported**
24 | `$/logTrace` | serverToClient | Yes | No | No | **Unsupported**
25 | `$/cancelRequest` | clientToServer | Yes | Yes | No | Intercepted directly by generated macro router (Partial support)
26 | `$/progress` | bidirectional | Yes | No | Yes | Client progress emission helper supported; incoming progress notifications unsupported

---

## 3. Structures

The LSP 3.18.0 meta-model defines **387 structures**. The generator correctly translates every structure into a Rust structure inside `tower-lsp-max-protocol/src/lsp_3_18.rs`. 

Notable 3.18.0 structure additions fully represented in the vocabulary:
- `InlineCompletionParams`, `InlineCompletionList`, `InlineCompletionItem`, `InlineCompletionRegistrationOptions`, `InlineCompletionContext`, `InlineCompletionOptions`, `InlineCompletionClientCapabilities`
- `TextDocumentContentParams`, `TextDocumentContentResult`, `TextDocumentContentRegistrationOptions`, `TextDocumentContentRefreshParams`, `TextDocumentContentOptions`, `TextDocumentContentClientCapabilities`
- `DocumentRangesFormattingParams`, `FoldingRangeWorkspaceClientCapabilities`, `CodeActionDisabled`, `LocationUriOnly`, `SnippetTextEdit`, `WorkspaceEditMetadata`

---

## 4. Enumerations

Below is the list of all **40 enumerations** defined in the LSP 3.18.0 meta-model. All 40 are generated as Rust enums or associated serialization helpers.

# | Enumeration Name | Status in Rust Crate | Notes
--- | --- | ---
1 | `ApplyKind` | Generated | New in 3.18.0
2 | `CodeActionKind` | Generated | Added `RefactorMove`, `Notebook` in 3.18
3 | `CodeActionTag` | Generated | New in 3.18.0
4 | `CodeActionTriggerKind` | Generated |
5 | `CompletionItemKind` | Generated |
6 | `CompletionItemTag` | Generated |
7 | `CompletionTriggerKind` | Generated |
8 | `DiagnosticSeverity` | Generated |
9 | `DiagnosticTag` | Generated |
10 | `DocumentDiagnosticReportKind` | Generated |
11 | `DocumentHighlightKind` | Generated |
12 | `ErrorCodes` | Generated |
13 | `FailureHandlingKind` | Generated |
14 | `FileChangeType` | Generated |
15 | `FileOperationPatternKind` | Generated |
16 | `FoldingRangeKind` | Generated |
17 | `InlayHintKind` | Generated |
18 | `InlineCompletionTriggerKind` | Generated | New in 3.18.0
19 | `InsertTextFormat` | Generated |
20 | `InsertTextMode` | Generated |
21 | `LSPErrorCodes` | Generated |
22 | `LanguageKind` | Generated | New in 3.18.0
23 | `MarkupKind` | Generated |
24 | `MessageType` | Generated | Added `Debug` (5) in 3.18
25 | `MonikerKind` | Generated |
26 | `NotebookCellKind` | Generated |
27 | `PositionEncodingKind` | Generated |
28 | `PrepareSupportDefaultBehavior` | Generated |
29 | `ResourceOperationKind` | Generated |
30 | `SemanticTokenModifiers` | Generated |
31 | `SemanticTokenTypes` | Generated | Added `label` in 3.18
32 | `SignatureHelpTriggerKind` | Generated |
33 | `SymbolKind` | Generated |
34 | `SymbolTag` | Generated |
35 | `TextDocumentSaveReason` | Generated |
36 | `TextDocumentSyncKind` | Generated |
37 | `TokenFormat` | Generated |
38 | `TraceValue` | Generated |
39 | `UniquenessLevel` | Generated |
40 | `WatchKind` | Generated |

---

## 5. Type Aliases

Below is the list of all **22 type aliases** defined in the LSP 3.18.0 meta-model. All 22 are mapped directly as Rust type aliases in `tower-lsp-max-protocol`.

# | Type Alias Name | Mapped Rust Type | Notes
--- | --- | ---
1 | `ChangeAnnotationIdentifier` | `pub type ChangeAnnotationIdentifier = String;` |
2 | `Declaration` | `pub type Declaration = LspAny;` | Lowered to `LspAny`
3 | `DeclarationLink` | `pub type DeclarationLink = LspAny;` | Lowered to `LspAny`
4 | `Definition` | `pub type Definition = LspAny;` | Lowered to `LspAny`
5 | `DefinitionLink` | `pub type DefinitionLink = LspAny;` | Lowered to `LspAny`
6 | `DocumentDiagnosticReport` | `pub type DocumentDiagnosticReport = LspAny;` | Lowered to `LspAny`
7 | `DocumentFilter` | `pub type DocumentFilter = LspAny;` | Lowered to `LspAny`
8 | `DocumentSelector` | `pub type DocumentSelector = Vec<DocumentFilter>;` |
9 | `GlobPattern` | `pub type GlobPattern = LspAny;` | Lowered to `LspAny`
10 | `InlineValue` | `pub type InlineValue = LspAny;` | Lowered to `LspAny`
11 | `LSPAny` | `pub type LSPAny = serde_json::Value;` |
12 | `LSPArray` | `pub type LSPArray = Vec<LSPAny>;` |
13 | `LSPObject` | `pub type LSPObject = serde_json::Map<String, LSPAny>;` |
14 | `MarkedString` | `pub type MarkedString = LspAny;` | Lowered to `LspAny`
15 | `NotebookDocumentFilter` | `pub type NotebookDocumentFilter = LspAny;` | Lowered to `LspAny`
16 | `Pattern` | `pub type Pattern = String;` |
17 | `PrepareRenameResult` | `pub type PrepareRenameResult = LspAny;` | Lowered to `LspAny`
18 | `ProgressToken` | `pub type ProgressToken = LspAny;` | Lowered to `LspAny`
19 | `RegularExpressionEngineKind` | `pub type RegularExpressionEngineKind = String;` |
20 | `TextDocumentContentChangeEvent` | `pub type TextDocumentContentChangeEvent = LspAny;` | Lowered to `LspAny`
21 | `TextDocumentFilter` | `pub type TextDocumentFilter = LspAny;` | Lowered to `LspAny`
22 | `WorkspaceDocumentDiagnosticReport` | `pub type WorkspaceDocumentDiagnosticReport = LspAny;` | Lowered to `LspAny`
