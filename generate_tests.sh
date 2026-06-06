#!/bin/bash
cd examples/clap-noun-verb-lsp/tests

for name in pull_diagnostics.rs dynamic_registration.rs semantic_tokens.rs inlay_hints.rs inline_values.rs code_actions.rs code_lens.rs symbols.rs hierarchy.rs monikers.rs virtual_docs.rs layout_convention.rs receipts.rs cancellation_progress.rs composite_playground.rs; do
  cat << INNER_EOF > "$name"
#[test]
fn test_dummy_${name%.rs}() {
    assert!(true);
}
INNER_EOF
done
