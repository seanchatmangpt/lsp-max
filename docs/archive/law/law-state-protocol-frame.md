# LSP State and Lifecycle Admission Law Framework

This document defines the semantic rules governing incoming message admission and lifecycle state transitions for the `lsp-max` server.

## Architecture

The server consists of four distinct architectural layers:

1. **Protocol (`lsp-max-protocol`):** Vocabulary definition and semantic types (e.g., Capability Vector, Diagnostics, Receipts).
2. **Server (`lsp-max`):** Tower service stack and JSON-RPC router.
3. **Runtime (`lsp-max-runtime`):** Process-level execution and snapshotting.
4. **Law Plugins (`lsp-max-agent`):** Out-of-band and in-band state conformance audits.

---

## Semantic Laws of LSP Lifecycle

### 1. Machine Match
The server's physical state machine must map 1-to-1 with the semantic states defined in the protocol ontology (`domain.ttl`):
*   `UninitializedState` -> `State::Uninitialized`
*   `InitializingState` -> `State::Initializing`
*   `InitializedState` -> `State::Initialized`
*   `ShutDownState` -> `State::ShutDown`
*   `ExitedState` -> `State::Exited`

*Verification Check:* Ensure the implementation transitions to `Initializing` immediately upon receiving `initialize` to prevent duplicate initialization attempts.

### 2. Total Selection
The input admission logic must comprehensively handle all possible state/request pairings:
*   In `Uninitialized` and `Initializing`: Reject normal messages. Accept `initialize`.
*   In `Initialized`: Accept normal messages and `shutdown`.
*   In `ShutDown`: Reject normal messages. Accept `exit`.
*   In `Exited`: Reject all messages.

### 3. Receipt Integrity
Admission transitions of transactional methods (e.g., `max/applyRepairTransaction`) must require and verify cryptographic `Receipt` tokens to guarantee that code modifications are atomic and recorded in-registry.

### 4. Exit Mapping
The terminal transition to `ExitedState` must map to the corresponding process exit status:
*   `0` (Success): If `exit` is called when the server is in `ShutDownState`.
*   `1` (Error): If `exit` is called when the server is in any other state.
