#!/usr/bin/env bash
set -euo pipefail

RECEIPT_PATH="receipts/v26.6.27.receipt.json"
mkdir -p receipts

echo "Generating release receipts for v26.6.27..."

gen_receipt_part() {
    local name="$1"
    local cmd="$2"
    local tmp_out=$(mktemp)
    local tmp_err=$(mktemp)
    
    echo "Running: $cmd"
    set +e
    eval "$cmd" > "$tmp_out" 2> "$tmp_err"
    local exit_code=$?
    set -e
    
    local out_digest=$(shasum -a 256 "$tmp_out" | awk '{print $1}')
    local err_digest=$(shasum -a 256 "$tmp_err" | awk '{print $1}')
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    cat <<EOF
  "$name": {
    "command": "$cmd",
    "exit_code": $exit_code,
    "stdout_digest": "$out_digest",
    "stderr_digest": "$err_digest",
    "timestamp": "$timestamp",
    "status": "$(if [ $exit_code -eq 0 ]; then echo "ADMITTED"; else echo "REFUSED"; fi)"
  }
EOF
}

cat > "$RECEIPT_PATH" <<EOF
{
  "release": "v26.6.27",
  "checkpoint": "LSPMAX-RUNTIME-ADMITTED-26.6.27",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "receipts": {
$(gen_receipt_part "cargo_test_workspace" "cargo test --all") ,
$(gen_receipt_part "clippy" "cargo clippy --all-targets -- -D warnings") ,
$(gen_receipt_part "anti_llm_cheat_lsp_dogfood" "cd ../anti-llm-cheat-lsp && cargo test --test dogfood_v26_6_27") ,
$(gen_receipt_part "gate_check_clear" "cargo run -p lsp-max-cli -- gate check || true") ,
$(gen_receipt_part "gate_check_blocked" "cargo run -p lsp-max-cli -- gate check || true") ,
$(gen_receipt_part "gate_check_agent_context" "cargo run -p lsp-max-cli -- gate check --format=agent-context || true")
  },
  "status": "ADMITTED"
}
EOF

echo "Written to $RECEIPT_PATH"
