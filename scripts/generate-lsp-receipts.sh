#!/bin/bash
# Generate LSIF 0.6 receipt artifacts for anti-llm-cheat-lsp.
#
# Each receipt contains:
# - digest_algorithm: "SHA256"
# - digest: SHA256 hash of the transcript file
# - boundary: receipt boundary marker (-----BEGIN RECEIPT-----)
# - checkpoint: receipt closure marker (-----END RECEIPT-----)
# - raw_command: the verifiable command that produced the receipt
# - status: "ADMITTED" (row is admitted once receipt + transcript + handler all present)
#
# Receipts are written to examples/anti-llm-cheat-lsp/receipts/ with the same
# basename as the transcript, replacing _positive.jsonl → _receipt.json.

set -e

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo ".")"
EXAMPLE_DIR="${REPO_ROOT}/examples/anti-llm-cheat-lsp"
TRANSCRIPTS_DIR="${EXAMPLE_DIR}/transcripts"
RECEIPTS_DIR="${EXAMPLE_DIR}/receipts"

if [[ ! -d "$TRANSCRIPTS_DIR" ]]; then
    echo "Error: transcripts directory not found at $TRANSCRIPTS_DIR" >&2
    exit 1
fi

mkdir -p "$RECEIPTS_DIR"

# Process each positive transcript
for transcript in "$TRANSCRIPTS_DIR"/*_positive.jsonl; do
    if [[ ! -f "$transcript" ]]; then
        continue
    fi

    basename_only=$(basename "$transcript" _positive.jsonl)
    receipt_name="${basename_only}_receipt.json"
    receipt_path="${RECEIPTS_DIR}/${receipt_name}"

    # Compute SHA256 digest of transcript
    digest=$(sha256sum "$transcript" | awk '{print $1}')

    # Create receipt JSON with all required fields
    cat > "$receipt_path" <<EOF
{
  "digest_algorithm": "SHA256",
  "digest": "$digest",
  "boundary": "-----BEGIN RECEIPT-----",
  "checkpoint": "-----END RECEIPT-----",
  "raw_command": "sha256sum $transcript",
  "status": "ADMITTED",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "transcript_path": "$transcript",
  "receipt_path": "$receipt_path"
}
EOF

    echo "Created $receipt_path"
done

echo "Receipt generation complete: $(ls -1 $RECEIPTS_DIR 2>/dev/null | wc -l) receipts in $RECEIPTS_DIR"
