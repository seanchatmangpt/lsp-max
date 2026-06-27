#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

mkdir -p .antigravity

# Stop guard is opt-in.
# Create this file before an overnight loop:
#   touch .antigravity/keep-running
#
# Remove it to allow normal stopping:
#   rm -f .antigravity/keep-running
if [ ! -f ".antigravity/keep-running" ]; then
  exit 0
fi

# Manual escape hatch.
if [ -f ".antigravity/allow-stop" ]; then
  exit 0
fi

# If the repo has no failset script yet, block stop and ask for the doctor surface.
if [ ! -x "./scripts/failset.sh" ]; then
  cat >&2 <<'EOF'
\[
q_{stop}=0
\]

\[
\mathbf f_{stop}=\{missing:scripts/failset.sh\}
\]

\[
\mu_{next}=repair(doctor,failset,q,receipts)
\]

ReturnOnly:
Agent=A_i
FilesChanged=\{\dots\}
CommandsRun=\{\dots\}
Receipts=\{\dots\}
\|\mathbf f_t\|_0=k
q_t\in\{0,1\}
\Delta\|\mathbf f\|_0=m
EOF
  exit 1
fi

RAW="$("./scripts/failset.sh" 2>/dev/null || echo 1)"
K="$(printf '%s\n' "$RAW" | tr -cd '0-9' | head -c 12)"
K="${K:-1}"

if [ "$K" = "0" ]; then
  exit 0
fi

cat >&2 <<EOF
\[
q_{stop}=0
\]

\[
\|\mathbf f_t\|_0=${K}
\]

\[
q_{orchestrator}=0
\]

\[
\mu_{next}=\arg\min_{\mu_i}\|\mathbf f(x_t+\mu_i(x_t))\|_0
\]

Run:
just doctor
just doctor-strict
just failset
just receipts-check

ReturnOnly:
Agent=A_i
FilesChanged=\{\dots\}
CommandsRun=\{\dots\}
Receipts=\{\dots\}
\|\mathbf f_t\|_0=k
q_t\in\{0,1\}
\Delta\|\mathbf f\|_0=m
EOF

exit 1
