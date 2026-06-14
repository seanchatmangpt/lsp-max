# MAX-001 LSP Surface Comparator Agent Report

## Status
MAX_IMPLEMENTATION_COMPLETE

## Overview
This report documents the surface comparison between the Language Server Protocol (LSP) version 3.18.0 meta-model, the generated Rust types, and the actual server implementation/routing in `lsp-max`. 

The primary objective of the LSP Surface Comparator is to map the protocol's vocabulary against the server's behavioral capabilities, verifying compliance and highlighting gaps.

## The Critical Distinction: Vocabulary vs. Routing vs. Implementation Coverage
When evaluating LSP compatibility, a simple "supported" flag is insufficient. We define three tiers of protocol coverage:
1. **Vocabulary Coverage:** Whether the protocol's requests, notifications, structures, enumerations, and type aliases are represented as Rust types in the codebase.
   - *Status:* **100% complete**. All types defined in the 3.18.0 meta-model are generated inside `lsp-max-protocol/src/lsp_3_18.rs`.
2. **Routing Coverage:** Whether the server registers a JSON-RPC method handler (via the `LanguageServer` trait and macro generation) so that incoming requests are accepted by the router.
   - *Status:* **Partial**. 66 standard LSP methods and 11 custom `max/` extension methods are registered. However, routing a method that returns `method_not_found` is a routing placeholder, not a functional implementation.
3. **Implementation Coverage:** Whether a routed method has a concrete implementation that obeys the semantic contract of the LSP specification, rather than returning a placeholder error or behaving as a stub.
   - *Status:* **Developing**. Standard LSP methods from older versions are fully implemented, whereas new LSP 3.18.0 methods (e.g., `textDocument/inlineCompletion`, `workspace/textDocumentContent`) are currently routed but unimplemented, returning `Error::method_not_found()`.

---

## Meta-Model Surface Area Metrics
*   **Fixture File:** `crates/lsp-max-specgen/fixtures/metaModel-3.18.json` (LSP 3.18.0)
*   **Requests:** 69
*   **Notifications:** 26
*   **Structures:** 387
*   **Enumerations:** 40
*   **Type Aliases:** 22

---

## Explicit Answers to Compliance Checklist

### Q1: What requests exist in the meta-model?
There are 69 requests defined in the LSP 3.18.0 meta-model. These cover all client-to-server actions (e.g. `textDocument/completion`) and server-to-client actions (e.g. `workspace/configuration`). (The full list is enumerated in `docs/reports/LSP-3.18-SURFACE-COMPARISON.md`).

### Q2: What notifications exist?
There are 26 notifications defined in the meta-model, such as `textDocument/didOpen` (client-to-server) and `window/showMessage` (server-to-client).

### Q3: What structures exist?
There are 387 structures defined in the meta-model. These include all parameter structures, result payloads, options, and capability blocks (e.g., `InitializeParams`, `InlineCompletionParams`, `MarkupContent`).

### Q4: What enumerations exist?
There are 40 enumerations in the meta-model, defining protocol kinds, tags, and trigger categories (e.g., `ApplyKind`, `CodeActionKind`, `InlineCompletionTriggerKind`).

### Q5: What type aliases exist?
There are 22 type aliases, mapping alternative representations or simplified forms (e.g., `DocumentSelector`, `ProgressToken`, `LSPAny`).

### Q6: Which are represented in generated Rust?
**All 69 requests, 26 notifications, 387 structures, 40 enumerations, and 22 type aliases are fully represented** as Rust types/constants in `lsp-max-protocol/src/lsp_3_18.rs` and its identical counterpart `generated/lsp_3_18.rs`.

### Q7: Which are routed in server code?
The server routes **66 standard LSP methods** (54 requests, 12 notifications) via the `LanguageServer` trait annotation in `src/lib.rs`. The generated router also intercepts and routes `$/cancelRequest` and `exit` directly.

### Q8: Which are exposed only as types but not handlers?
1. **Server-to-Client methods** that are exposed via helper methods on the `Client` struct but do not have server-side handlers:
   - *Requests:* `client/registerCapability`, `client/unregisterCapability`, `window/showDocument`, `window/showMessageRequest`, `workspace/applyEdit`, `workspace/codeLens/refresh`, `workspace/configuration`, `workspace/diagnostic/refresh`, `workspace/inlayHint/refresh`, `workspace/inlineValue/refresh`, `workspace/semanticTokens/refresh`, `workspace/workspaceFolders`.
   - *Notifications:* `telemetry/event`, `textDocument/publishDiagnostics`, `window/logMessage`, `window/showMessage`.
2. **Server-to-Client methods omitted from Client helpers** (exposed strictly as generated types in the protocol library):
   - `window/workDoneProgress/create`
   - `workspace/foldingRange/refresh`
   - `$/logTrace`
3. **Client-to-Server methods omitted from Server Handlers** (exposed only as generated types):
   - `textDocument/rangesFormatting` (New in LSP 3.18.0)
   - `$/setTrace`
   - `window/workDoneProgress/cancel`
   - Notebook document notifications: `notebookDocument/didOpen`, `notebookDocument/didChange`, `notebookDocument/didSave`, `notebookDocument/didClose`
   - `$/progress` (Only client-to-server direction is unsupported; server-to-client progress emission is supported).

### Q9: Which are intentionally unsupported?
The following are marked as unsupported in `FEATURES.md` or annotated as unsupported in code comments:
- **Work Done Progress:** `window/workDoneProgress/create` and `window/workDoneProgress/cancel`
- **Notebook Support:** `notebookDocument/didOpen`, `notebookDocument/didChange`, `notebookDocument/didSave`, `notebookDocument/didClose`
- **Tracing Support:** `$/setTrace`, `$/logTrace`
- **Partial/Omitted:** `$/progress` (standard progress mechanism is unsupported, though client progress building is partially supported), `textDocument/rangesFormatting` (new formatting method is unrouted).

---

## Required Follow-up Gates
To evolve `lsp-max` beyond vocabulary and basic routing coverage:
1. **Implement 3.18.0 Handlers:** Upgrade the stub implementations of `textDocument/inlineCompletion` and `workspace/textDocumentContent` to provide real semantic capabilities.
2. **Standardize Server-to-Client Refresh Handling:** Resolve the direction mismatch on `workspace/textDocumentContent/refresh`, which is currently routed as an incoming handler returning `method_not_found`, but should be implemented as an outgoing client request.
3. **Notebook and Tracing Parity:** Re-evaluate the status of notebook document notifications and tracing APIs, implementing handlers if downstream clients require them.
