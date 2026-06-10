# Law-State Runtime Primitives and Machine Transitions

This document details the low-level execution semantics, typestate representations, and cryptographic transition laws enforced by `lsp-max-runtime`.

---

## The Generic Typestate Machine: `Machine<L, P, D>`

The core of the `lsp-max-runtime` engine is the generic typestate machine:

```rust
pub struct Machine<L: Law, P: Phase, D: Data> {
    pub _law: PhantomData<L>,
    pub phase: P,
    pub data: D,
}
```

This structural container ensures that state transitions are verified at both compile time and runtime:
- **`L: Law`**: The active governance policy (e.g., `AccessAdmissionLaw`) that specifies transition validation rules.
- **`P: Phase`**: The compile-time representation of the machine's current operational state.
- **`D: Data`**: The underlying state payload carried during this specific phase.

---

## Semantic Lifecycle Transitions

The server moves through a sequence of five distinct, unidirectional phases. Each transition is governed by the `TypestateKernel` trait:

```rust
pub trait TypestateKernel<L: Law, P: Phase, D: Data> {
    type Input;
    type OutputPhase: Phase;
    type OutputData: Data;
    type Receipt;

    fn validate(&self, input: &Self::Input) -> Result<(), L::Error>;
    fn select(&self, input: &Self::Input) -> Self::OutputPhase;
    fn admit(self, input: Self::Input) -> Result<Machine<L, Self::OutputPhase, Self::OutputData>, L::Error>;
    fn receipt(&self) -> Self::Receipt;
    fn exit(self) -> D;
    fn replay(history: Vec<Self::Receipt>) -> Self;
}
```

The phase lifecycle is defined by the following transitions:

```
[Uninitialized]
       │
       ▼ (admit: initialize)
[Initializing]
       │
       ▼ (admit: initialized)
[Initialized]  ◄───►  [Operational Transactions / Custom RPC]
       │
       ▼ (admit: shutdown)
[ShutDown]
       │
       ▼ (admit: exit)
[Exited]
```

### 1. `Uninitialized`
*   **Semantic Phase:** `UninitializedState`
*   **Data Payload:** `EmptyData`
*   **Admission Rules:** Rejects all standard JSON-RPC messages. Accepts only the `initialize` request.
*   **Transition Target:** `Initializing`

### 2. `Initializing`
*   **Semantic Phase:** `InitializingState`
*   **Data Payload:** `InitializingData` (storing `client_capabilities`)
*   **Admission Rules:** Processing configuration options and capability negotiation. Rejects standard project actions.
*   **Transition Target:** `Initialized`

### 3. `Initialized`
*   **Semantic Phase:** `InitializedState`
*   **Data Payload:** `InitializedData` (storing client and negotiated server capabilities)
*   **Admission Rules:** Fully active operational phase. Admits custom `max/` RPC operations, edits, and gate checks.
*   **Transition Target:** `ShutDown`

### 4. `ShutDown`
*   **Semantic Phase:** `ShutDownState`
*   **Data Payload:** `EmptyData`
*   **Admission Rules:** Rejects all custom and standard LSP transactions. Admits only the `exit` notification.
*   **Transition Target:** `Exited`

### 5. `Exited`
*   **Semantic Phase:** `ExitedState`
*   **Data Payload:** `EmptyData`
*   **Admission Rules:** Terminal state. Rejects all messages. The server shuts down its internal listener loops.

---

## Process Exit Status Mapping

The transition from the penultimate phase to the terminal state maps directly to the host operating system's exit status. This guarantees that process exit behavior is a deterministic function of the state machine history.

*   **Clean Exit (Exit Code `0`):** Initiated if the process receives the `exit` notification when the machine is in the `ShutDownState` phase. This signifies that the client initiated and completed a standard, graceful lifecycle negotiation sequence.
*   **Unclean Exit (Exit Code `1`):** Triggered if the process terminates or receives the `exit` notification from **any state other than** `ShutDownState`. Any sudden termination, unhandled panic, or protocol violation that interrupts the state flow will force an exit code of `1` to signal to supervising orchestrator agents that the protocol state machine crashed.

---

## Cryptographic Receipt Logs and Transition History

To mathematically guarantee the history of state transitions, the system utilizes cryptographic receipt logs. Every state transition computes a unique `Receipt` structure containing:
*   `receipt_id`: An identifier of the specific transition path.
*   `hash`: A cryptographic checksum representing the system state and the previous transition hash, forming a linear **hash chain**.

```rust
pub struct Receipt {
    pub receipt_id: String,
    pub hash: String,
}
```

### Mathematical State Reconstruction
Using the `replay` function on the `TypestateKernel`, the runtime can ingest a historical ledger of these receipt blocks:
1.  **Bootstrapping:** The kernel initializes to `Uninitialized`.
2.  **Iterative Replay:** For each receipt in the history, the kernel validates the cryptographic hash chain. If a hash mismatch occurs, replay is aborted, marking the history as tampered or corrupted.
3.  **Restoration:** Upon successful validation, the machine is restored to the exact, verified phase and data configuration matching the ledger's end.

This mechanism ensures that the workspace state is tamper-proof and that all historic actions can be audited out-of-band by independent validator entities.
