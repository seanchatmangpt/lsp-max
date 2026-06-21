/// NEGATIVE CONTROL: unknown_collapse.rs
///
/// This file demonstrates the law violation that SCAFFOLD-AXIS-001 detects:
/// coercing an UNKNOWN axis directly to ADMITTED without evidence.
///
/// An axis may only become ADMITTED after all three preconditions are met:
///   1. A receipt exists in `receipts/<method>.json` with a valid BLAKE3 digest
///   2. A transcript exists in `transcripts/<method>.jsonl`
///   3. A negative-control exists in `fixtures/negative_controls/<method>.rs`
///      that demonstrates the law violation the receipt guards against
///
/// The anti-llm-cheat-lsp canary emits SCAFFOLD-AXIS-001 when it detects this
/// pattern in any lsp-max crate.

#[cfg(test)]
mod negative_control {
    use lsp_max_scaffold::law::{ScaffoldAxis, ScaffoldConformanceVector};

    /// ANTI-PATTERN: Calling `admit_axis` without first producing a receipt,
    /// transcript, and negative-control. This is the exact violation that the
    /// admission law guards against.
    ///
    /// In the real system, this would be caught at review time by:
    ///   - `lsp-max-scaffold admit check --method textDocument/hover`
    ///   - The ANDON gate blocking CI until all axes have receipts
    ///   - The anti-llm-cheat-lsp canary emitting SCAFFOLD-AXIS-001
    #[test]
    fn anti_pattern_bare_admit_without_evidence() {
        let mut v = ScaffoldConformanceVector::new();

        // VIOLATION: admitting an axis without a receipt chain.
        // This test PASSES (the code compiles), but the axis promotion is
        // untrustworthy — it lacks the evidence chain required by law.
        let admitted = v.admit_axis(ScaffoldAxis::Receipt);
        assert!(admitted, "admit_axis accepted the axis — LAW VIOLATION UNDETECTED AT RUNTIME");

        // The consequence: status_label returns "PARTIAL" (some still unknown)
        // rather than correctly remaining "PARTIAL" because Receipt is now
        // erroneously admitted. A receipt-backed system would prevent this.
        assert!(v.admitted.contains(&ScaffoldAxis::Receipt));
    }
}
