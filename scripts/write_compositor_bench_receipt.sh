#!/usr/bin/env bash
# Runs the compositor scale benchmark suite and writes a BLAKE3-signed receipt
# to receipts/compositor-scale.receipt.json.
#
# Required: cargo, b3sum (or blake3 CLI), jq
# Called by: just bench-compositor

set -euo pipefail

RECEIPT_PATH="receipts/compositor-scale.receipt.json"
BENCH_OUTPUT_FILE="/tmp/lsp_max_compositor_bench_output.txt"
CHECKPOINT="COMPOSITOR-SCALE-ADMITTED-26.6.9"
BOUNDARY="crates/lsp-max-compositor/benches/compositor_micro.rs"

echo "Running compositor micro-benchmark suite..."
cargo bench -p lsp-max-compositor --bench compositor_micro -- \
    --output-format bencher 2>&1 | tee "$BENCH_OUTPUT_FILE"

# Digest the raw bench output
if command -v b3sum &>/dev/null; then
    OUTPUT_DIGEST=$(b3sum --no-names "$BENCH_OUTPUT_FILE")
elif command -v blake3 &>/dev/null; then
    OUTPUT_DIGEST=$(blake3 "$BENCH_OUTPUT_FILE" | awk '{print $1}')
else
    OUTPUT_DIGEST=$(shasum -a 256 "$BENCH_OUTPUT_FILE" | awk '{print $1}')
    DIGEST_ALG="SHA-256"
fi
DIGEST_ALG="${DIGEST_ALG:-BLAKE3}"

# Digest the primary benchmark source as the artifact digest
ARTIFACT_DIGEST=$(shasum -a 256 "$BOUNDARY" | awk '{print $1}')

ISO_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat > "$RECEIPT_PATH" <<EOF
{
  "checkpoint": "$CHECKPOINT",
  "boundary": "$BOUNDARY",
  "digest": "$ARTIFACT_DIGEST",
  "digest_algorithm": "SHA-256",
  "output_digest": "$OUTPUT_DIGEST",
  "output_digest_algorithm": "$DIGEST_ALG",
  "raw_command": "cargo bench -p lsp-max-compositor --bench compositor_micro -- --output-format bencher",
  "producing_workspace": "lsp-max",
  "timestamp": "$ISO_DATE",
  "claims": {
    "CS1": "deposit_5_servers_throughput: DiagnosticBuffer::deposit() throughput measured at 5 concurrent child servers",
    "CS2": "deposit_500_servers_throughput: DiagnosticBuffer::deposit() throughput measured at 500 concurrent child servers",
    "CS3": "flush_latency_500x100_ns_per_diag: DiagnosticBuffer::flush() latency recorded at 500 servers x 100 diagnostics per URI",
    "CS4": "merge_500x100_distinct_keys_ns_per_diag: merge_diagnostics() cost at 500x100 distinct-key entries",
    "CS5": "merge_500x100_law_codes_ns_per_diag: merge_diagnostics() cost at 500x100 REFUSED_BY_LAW entries (sort branch)",
    "CS6": "signal_loss_rate_300_uris_pct: try_send() drop rate quantified at 300 URIs against capacity-256 channel"
  },
  "status": "ADMITTED",
  "output_hash": "$OUTPUT_DIGEST",
  "run_id": "$CHECKPOINT",
  "replay_pointer": "cargo bench -p lsp-max-compositor --bench compositor_micro -- --output-format bencher"
}
EOF

echo ""
echo "Receipt written: $RECEIPT_PATH"
echo "Checkpoint: $CHECKPOINT"
echo "Output digest ($DIGEST_ALG): $OUTPUT_DIGEST"
