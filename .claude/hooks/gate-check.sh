#!/bin/bash
# PreToolUse ANDON gate. Exit 0 = proceed. Exit 1 = blocked with structured reason.
# Passes through when lsp-max-cli is not yet built (fresh session).
command -v lsp-max-cli > /dev/null 2>&1 || exit 0

lsp-max-cli gate check > /dev/null 2>&1
gate_exit=$?

if [ $gate_exit -eq 0 ]; then
  exit 0
fi

# Gate is blocked — emit structured JSON with reason and first violation if available
fitness_file="${FITNESS_PATH:-${CLAUDE_PROJECT_DIR:-.}/.claude/lsp-max-fitness.json}"

first_violation="null"
reason="ANDON gate is active"

if [ -f "$fitness_file" ]; then
  # Extract first violation if present
  first_violation=$(python3 -c "
import json, sys
try:
    with open('$fitness_file') as f:
        d = json.load(f)
    viols = d.get('violations', [])
    if viols:
        print(json.dumps(viols[0]))
    else:
        print('null')
except Exception as e:
    print('null')
" 2>/dev/null || echo "null")
fi

if [ "$first_violation" = "null" ]; then
  cat <<EOF
{"decision":"block","reason":"ANDON gate is active — lsp-max-cli gate check returned non-zero","routing_action":"halt_until_gate_clears"}
EOF
else
  python3 -c "
import json, sys
fv = $first_violation
out = {
    'decision': 'block',
    'reason': 'ANDON gate is active — lsp-max-cli gate check returned non-zero',
    'routing_action': 'halt_until_gate_clears',
    'first_violation': fv
}
print(json.dumps(out))
" 2>/dev/null || cat <<EOF
{"decision":"block","reason":"ANDON gate is active — lsp-max-cli gate check returned non-zero","routing_action":"halt_until_gate_clears"}
EOF
fi

exit 1
