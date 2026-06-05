# Max Protocol Law: Access, Diagnostic Refusal, and Repair Transactions

This document defines the custom JSON-RPC protocol interface implemented by `tower-lsp-max` in the `max/` namespace. These interfaces govern state queries, diagnostic explanations, atomic repair transactions, correctness gates, and verification receipts.

---

## Schema Rules for the `max/` Namespace

All custom RPC methods implemented by `tower-lsp-max` are namespaced under the `max/` prefix. They enforce strict type-safe schemas for inputs and responses. These endpoints are designed to interact with autonomous agents to query, inspect, and transition the project state via verifiable, cryptographic transaction mechanisms.

---

## Custom JSON-RPC Methods

### 1. `max/snapshot`
*   **Method Name:** `max/snapshot`
*   **Parameters:** `()` (None)
*   **Response:** `SnapshotId` (a string-wrapped identifier, e.g. `SnapshotId(String)`)
*   **Description:** Captures the current resident state of the virtual workspace and returns a unique, deterministic `SnapshotId`. This snapshot serves as the state baseline for all subsequent queries and conformance calculations.

### 2. `max/conformanceVector`
*   **Method Name:** `max/conformanceVector`
*   **Parameters:** `SnapshotId`
*   **Response:** `ConformanceVector`
*   **Description:** Evaluates the conformance metrics of the workspace corresponding to the specified `SnapshotId`. The response returns the calculated compliance score (on a 100-point scale) and the enforcement mode status (`strict_mode: bool`).

### 3. `max/explainDiagnostic`
*   **Method Name:** `max/explainDiagnostic`
*   **Parameters:** `String` (the unique `diagnostic_id`)
*   **Response:** `MaxDiagnostic`
*   **Description:** Retrieves full rich metadata for a given active diagnostic, including violated laws, transition failures, documentation routes, and available automated repair actions.

### 4. `max/repairPlan`
*   **Method Name:** `max/repairPlan`
*   **Parameters:** `String` (either a `diagnostic_id` or a specific `law_id`)
*   **Response:** `Vec<MaxCodeAction>`
*   **Description:** Calculates and returns a prioritized list of atomic repair vectors (`MaxCodeAction` instances) that satisfy the preconditions to resolve the targeted diagnostic or violation.

### 5. `max/applyRepairTransaction`
*   **Method Name:** `max/applyRepairTransaction`
*   **Parameters:** `MaxCodeAction`
*   **Response:** `Receipt` (a cryptographic structure containing a unique receipt ID and block hash)
*   **Description:** Submits a `MaxCodeAction` transaction vector to the admission kernel. If preconditions and validation plans pass, the engine applies the state modification atomically and registers a cryptographic transaction `Receipt`.

### 6. `max/exportAnalysisBundle`
*   **Method Name:** `max/exportAnalysisBundle`
*   **Parameters:** `SnapshotId`
*   **Response:** `AnalysisBundle`
*   **Description:** Compiles and exports the complete metadata bundle for a given workspace snapshot. The bundle aggregates the capability vector, active diagnostics, available repair actions, compliance score, and historical receipts.

### 7. `max/runGate`
*   **Method Name:** `max/runGate`
*   **Parameters:** `GateId` (representing a specific correctness gate, e.g. `GateId(String)`)
*   **Response:** `bool` (indicating success or failure)
*   **Description:** Triggers the immediate out-of-band execution of a specific correctness validation gate. Returns `true` if the gate validates successfully, and `false` otherwise.

### 8. `max/clearDiagnostic`
*   **Method Name:** `max/clearDiagnostic`
*   **Parameters:** `String` (the target `diagnostic_id`)
*   **Response:** `()` (Empty)
*   **Description:** An administrative override that forcefully removes a diagnostic from the active state list. Use of this method bypasses normal verification gates and must be logged as an unverified state change.

### 9. `max/receipt`
*   **Method Name:** `max/receipt`
*   **Parameters:** `String` (the target `receipt_id`)
*   **Response:** `Receipt`
*   **Description:** Queries and retrieves the cryptographic receipt metadata (including the hash and identifier) for a previously committed transaction.

### 10. `max/releaseActuation`
*   **Method Name:** `max/releaseActuation`
*   **Parameters:** A JSON object:
    *   `instance_id` (String, required): The unique identifier of the target instance to actuate.
*   **Response:** `ReleaseActuationResult`:
    *   `released` (Boolean): True if the release succeeded.
    *   `instance_id` (String): The ID of the instance.
    *   `conformance_score` (Number): The current conformance score (e.g. 100.0).
    *   `conformance_grade` (String, optional in some contexts, but present in runtime): The corresponding letter grade for compliance.
    *   `release_receipt` (Receipt, optional): The cryptographic receipt proving the release transaction if successful.
*   **Description:** Evaluates and actuates a release for the specified instance.
*   **Behavior Rules:**
    *   Release is permitted if and only if there are no active diagnostics blocking conformance for the specified instance.
    *   A diagnostic blocks the instance if its `diagnostic_id` contains the `instance_id` substring (or exact match).
    *   Upon successful verification, the engine increments the sequence counter (`action_seq`), issues a unique release receipt (`rcpt-release-<instance_id>`), and appends a corresponding delta record to the `conformance_delta_log` before returning the response.
*   **Locking Mechanisms:** Acquires a write lock on the global `REGISTRY` state to thread-safely verify conformance/diagnostics, issue the receipt, update the sequence number, and write to the delta log.
*   **Error Paths:**
    *   Returns an error (e.g. `missing instance_id`) if parameter `instance_id` is absent.
    *   Returns an error if the specified instance cannot be found in the registry.
    *   Returns a transition/request failed error (e.g. `Release refused: <N> active diagnostics blocking conformance`) if there are outstanding violations blocking the release.

### 11. `max/conformanceDelta`
*   **Method Name:** `max/conformanceDelta`
*   **Parameters:** A JSON object:
    *   `since_seq` (Number, optional): The sequence cursor since which score updates should be retrieved. Defaults to 0 if omitted.
*   **Response:** A JSON object:
    *   `deltas` (Array of ConformanceDeltaEntry): A list of recent delta records, each containing:
        *   `seq` (Number): Monotonically increasing sequence number.
        *   `instance_id` (String): The identifier of the instance.
        *   `old_score` (Number): The prior conformance score.
        *   `new_score` (Number): The post-transition conformance score.
    *   `current_seq` (Number): The latest sequence number in the global registry state.
*   **Description:** Retrieves a sequence of conformance delta entries since the provided sequence number to support live dashboard status polling and audit trails.
*   **Behavior Rules:**
    *   Reads from `conformance_delta_log` (the single authoritative delta store) and filters for entries where `seq > since_seq`.
    *   The log capacity is bounded (e.g. up to 4096 entries) and behaves as a circular buffer, evicting the oldest entries when exceeded.
*   **Locking Mechanisms:** Acquires a read/write lock on the global `REGISTRY` state to inspect the circular `conformance_delta_log` buffer.
*   **Error Paths:**
    *   Defaults the `since_seq` cursor to `0` if parameters are malformed, or if `since_seq` is not a valid unsigned integer.

---

## Diagnostics as Refused Transitions

In the `tower-lsp-max` paradigm, a diagnostic is **not** a user-interface warning or a formatting suggestion. It is a formal **refusal by the admission kernel to transition state**. 

When an agent attempts to transition the codebase (e.g. committing a file change or invoking a pipeline transition), the admission kernel checks the active laws. If a violation is detected, a state transition is blocked, and a `MaxDiagnostic` is emitted.

### The `MaxDiagnostic` Structure
The structure of a diagnostic is formally defined as:
*   `lsp`: The standard `lsp_types::Diagnostic` struct containing range, severity, and text.
*   `diagnostic_id`: A unique UUID string identifying this specific diagnostic instance.
*   `law_id`: The identifier of the specific ontology rule or state law being violated.
*   `attempted_transition`: An optional `TransitionAttempt` struct outlining the source state and the failed target state.
*   `violated_axes`: A list of strings representing the conformance axes (e.g. Security, Structural, Semantic) that were violated.
*   `doc_routes`: A list of documentation paths (`DocRoute`) that define the governing law for this violation.
*   `repair_actions`: A list of pre-calculated `RepairAction` options (each with an `action_id` and `description`) representing paths to compliance.
*   `verification_gates`: A list of `GateId` requirements that must be executed to verify this diagnostic's resolution.
*   `receipt_obligation`: An optional list of cryptographic receipts (`ReceiptObligation`) required to clear the violation state.

---

## Code Actions as Repair Transactions

To resolve a refused transition, agents must execute atomic state repair vectors. Under `tower-lsp-max`, these are represented by the `MaxCodeAction` struct. Instead of generic text edits, they represent formal **state repair transactions** containing plans for verification, fallback, and logging.

### The `MaxCodeAction` Structure
Each `MaxCodeAction` consists of:
1.  **`action`**: The standard `lsp_types::CodeAction` containing the actual workspace changes (text edits, document changes).
2.  **`preconditions`**: A list of `Precondition` assertions (e.g., target file must exist, compiler state must be valid) that must hold true before the transaction is executed.
3.  **`validation_plan`**: A `ValidationPlan` containing a list of `GateId` gates that the system must run immediately after applying the edits to confirm correctness.
4.  **`rollback_plan`**: A `RollbackPlan` detailing the rollback strategy (e.g., git revert, workspace snapshot restore) to execute if any validation gate fails after application.
5.  **`receipt_plan`**: A `ReceiptPlan` detailing the list of expected cryptographic receipts that will be appended to the ledger upon successful transaction completion.
