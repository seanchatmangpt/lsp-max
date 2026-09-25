# Anti-Human Correctness Gates and Automated Verification Law

This document establishes the absolute law of automated verification for `tower-lsp-max` and outlaws human review as a validator of state correctness.

---

## The Rejection of Human Review

Under the post-human LSP doctrine, human review is formally declared to be **meaningless, non-binding, and ineligible to serve as a correctness gate**. Human inspection is subjective, error-prone, asynchronous, and cannot be cryptographically proven or verified by autonomous agents. 

State correctness and project compliance are determined **strictly and exclusively** through automated verification gates. If a state transition cannot be verified programmatically, it is incorrect. The server's admission kernel will refuse transitions that fail automated validation, regardless of human consensus, manual approvals, or code review signatures.

---

## Formally Defined Conformance Calculation

Every workspace state snapshot is evaluated to produce a deterministic compliance score. The score is calculated on a 100-point scale using the following mathematical formula:

$$\text{Score} = \max\left(0.0,\ 100.0 - \sum \text{Diagnostic Penalties}\right)$$

### Severity Penalty Scale

Each active diagnostic registered in the state-machine is categorized by its LSP severity. Penalties are deducted from the conformance score based on the following scale:

| Diagnostic Severity | Penalty Value | Description / Context |
| :--- | :--- | :--- |
| **Error** | `30.0` | Severe violation of typestate rules, AST invalidity, or gate failure. |
| **Warning** | `15.0` | Minor architectural law drift or deprecated state structures. |
| **Information** | `5.0` | Suggestive schema mismatches or non-blocking ontology updates. |
| **Hint** | `5.0` | Minor code/documentation style variations or non-standard patterns. |

### Correctness Condition
For a project state to be considered correct, compliant, and ready for deployment or integration, the Conformance Score must be **exactly 100.0**. Any score less than 100.0 indicates a state of non-conformance, and the admission kernel will reject any transitions attempting to write this state to production or log it as a verified historic transition.

---

## Integration of Verification Gates and Receipt Obligations

To resolve diagnostics and transition the system back to a Conformance Score of 100.0, the system integrates verification gates and receipt obligations.

```
       +---------------------------------------------+
       |   Code Modification / Repair Transaction    |
       +---------------------------------------------+
                              │
                              ▼
       +---------------------------------------------+
       |    Run Verification Gates (GateId checks)    |
       +---------------------------------------------+
                              │
               ┌──────────────┴──────────────┐
               ▼ (Pass)                      ▼ (Fail)
    +----------------------+       +-------------------------+
    | Check Receipt Oblig. |       | Rollback to prev state  |
    +----------------------+       | Emit MaxDiagnostic      |
               │                   +-------------------------+
               ▼
    +----------------------+
    | Log Cryptographic    |
    | Receipt Ledger       |
    +----------------------+
```

### 1. Verification Gates (`GateId`)
When an agent attempts a state repair transaction (via `max/applyRepairTransaction`), it must provide a `validation_plan` listing the `GateId` values that must be executed. 
*   **Gate Execution:** The server executes the specified gates (e.g., compile checks, unit test gates, ontology linting) out-of-band.
*   **Success Criterion:** If all gates return `true`, the diagnostic is cleared.
*   **Failure Protocol:** If any gate returns `false`, the transaction is rolled back, the conformance score is updated with the corresponding diagnostic penalty, and the state transition is refused.

### 2. Receipt Obligations (`ReceiptObligation`)
A diagnostic can specify a `receipt_obligation` containing a list of `required_receipts`. This requires the agent to present cryptographic receipts of preceding, successfully validated transactions.
*   **Validation of Proofs:** The kernel checks the hash signatures on the provided receipts to verify that they are linked to the current state's history ledger.
*   **State Admission:** If the receipt signatures are valid and cover the necessary obligations, the transition is admitted.
*   **Rejection:** If the receipts are missing, invalid, or have mismatched hash signatures, the admission kernel rejects the state transition.
