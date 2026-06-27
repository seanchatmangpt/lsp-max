#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

GATE_JSON="scripts/v26-gate.json"

if [ ! -f "$GATE_JSON" ]; then
    cat <<EOF
{
  "release": "v26.6.28",
  "q_release": 0,
  "failset_cardinality": 1,
  "counterexamples": ["v26-gate.json missing"],
  "components": {}
}
EOF
    exit 1
fi

python3 -c '
import json, os, sys
data = json.load(open("scripts/v26-gate.json"))
missing = []
for d in data.get("detectors", []):
    receipt = d.get("receipt_file")
    if not os.path.exists(receipt):
        missing.append(d["name"])
        
q_release = 1 if not missing else 0
failset = len(missing)

out = {
  "release": "v26.6.28",
  "q_release": q_release,
  "failset_cardinality": failset,
  "counterexamples": missing,
  "components": {}
}

print(json.dumps(out, indent=2))
if missing:
    sys.exit(1)
'
