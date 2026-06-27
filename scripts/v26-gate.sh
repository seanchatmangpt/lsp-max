#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

GATE_JSON="scripts/v26-gate.json"

if [ ! -f "$GATE_JSON" ]; then
    echo "\["
    echo "q_{release}=0"
    echo "reason=v26-gate.json missing"
    echo "\]"
    exit 1
fi

MISSING=0
total_detectors=$(python3 -c 'import json; print(len(json.load(open("scripts/v26-gate.json"))["detectors"]))')
checked=0

echo "Evaluating v26-gate JSON..."

python3 -c '
import json, os, sys
data = json.load(open("scripts/v26-gate.json"))
missing = []
for d in data.get("detectors", []):
    receipt = d.get("receipt_file")
    if not os.path.exists(receipt):
        missing.append(d["name"])
if missing:
    print("Missing receipts for:", ", ".join(missing))
    sys.exit(1)
' || MISSING=1

if [ "$MISSING" -eq 1 ]; then
    echo "\["
    echo "q_{release}=0"
    echo "\]"
    exit 1
else
    echo "\["
    echo "q_{release}=1"
    echo "\]"
    exit 0
fi
