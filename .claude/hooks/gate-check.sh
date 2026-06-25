#!/bin/bash
# PreToolUse ANDON gate. Exit 0 = proceed. Exit 1 = blocked with structured reason.
# Passes through when lsp-max-cli is not yet built (fresh session).
command -v lsp-max-cli >/dev/null 2>&1 || exit 0

if lsp-max-cli gate check 2>/dev/null; then
  exit 0
fi

# Gate is active — emit structured reason so the agent can self-correct.
FITNESS_FILE="${CLAUDE_PROJECT_DIR:-.}/.claude/lsp-max-fitness.json"
if command -v python3 >/dev/null 2>&1 && [[ -f "$FITNESS_FILE" ]]; then
  FITNESS_PATH="$FITNESS_FILE" python3 -c "
import json, os
try:
    d = json.load(open(os.environ['FITNESS_PATH']))
    v0 = d.get('violations', [{}])[0]
    out = {
        'decision': 'block',
        'reason': 'ANDON active — law_status={} fitness={:.3f} violations={}'.format(
            d.get('law_status', 'UNKNOWN'),
            d.get('fitness', 0.0),
            d.get('declare_violations', '?')
        ),
        'routing_action': 'call lsp_violations to read full list, then lsp_repair_plan with the constraint'
    }
    if v0:
        out['first_violation'] = {
            'constraint': v0.get('constraint', ''),
            'detail': v0.get('detail', '')
        }
    print(json.dumps(out))
except Exception:
    print('{\"decision\":\"block\",\"reason\":\"ANDON gate active\",\"routing_action\":\"run lsp-max-cli gate list\"}')
"
else
  echo '{"decision":"block","reason":"ANDON gate active","routing_action":"run lsp-max-cli gate list"}'
fi
exit 1
