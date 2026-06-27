#!/usr/bin/env bash
set -euo pipefail

HOOK="${1:-unknown}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

mkdir -p .antigravity/ocel .antigravity/payloads

TS="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
NS="$(date -u +"%Y%m%dT%H%M%S").$$"
EVENT_ID="${NS}.${HOOK}"
PAYLOAD=".antigravity/payloads/${EVENT_ID}.stdin"
EVENT=".antigravity/ocel/${EVENT_ID}.json"
JSONL=".antigravity/ocel/events.jsonl"

cat > "$PAYLOAD" || true

python3 - "$HOOK" "$TS" "$EVENT_ID" "$PAYLOAD" "$EVENT" "$JSONL" <<'PY'
import json
import pathlib
import subprocess
import sys
import hashlib

hook, ts, event_id, payload_path, event_path, jsonl_path = sys.argv[1:]

def sh(cmd):
    try:
        return subprocess.check_output(
            cmd,
            shell=True,
            text=True,
            stderr=subprocess.STDOUT
        ).splitlines()
    except subprocess.CalledProcessError as e:
        return e.output.splitlines()
    except Exception as e:
        return [f"ERROR:{e}"]

def sha_file(path):
    p = pathlib.Path(path)
    if not p.exists():
        return None
    return hashlib.sha256(p.read_bytes()).hexdigest()

def parse_numstat(lines):
    out = []
    total_added = 0
    total_deleted = 0

    for line in lines:
        parts = line.split("\t")
        if len(parts) < 3:
            continue

        added_raw, deleted_raw, file_path = parts[0], parts[1], parts[2]

        added = 0 if added_raw == "-" else int(added_raw or 0)
        deleted = 0 if deleted_raw == "-" else int(deleted_raw or 0)

        total_added += added
        total_deleted += deleted

        out.append({
            "file": file_path,
            "added": added,
            "deleted": deleted
        })

    return out, total_added, total_deleted

status_lines = sh("git status --porcelain=v1")
diff_files = sh("git diff --name-only")
cached_files = sh("git diff --cached --name-only")
untracked_files = sh("git ls-files --others --exclude-standard")

numstat_lines = sh("git diff --numstat")
cached_numstat_lines = sh("git diff --cached --numstat")

numstat, added, deleted = parse_numstat(numstat_lines)
cached_numstat, cached_added, cached_deleted = parse_numstat(cached_numstat_lines)

changed_files = sorted(set(diff_files + cached_files + untracked_files))

head = sh("git rev-parse HEAD")
branch = sh("git branch --show-current")

payload_sha256 = sha_file(payload_path)

event = {
    "ocel:version": "2.0",
    "event_id": event_id,
    "event_type": hook,
    "activity": hook,
    "timestamp": ts,
    "objects": [
        {
            "type": "repository",
            "id": "lsp-max"
        },
        *[
            {
                "type": "file",
                "id": f
            }
            for f in changed_files
        ]
    ],
    "attributes": {
        "git": {
            "head": head[0] if head else None,
            "branch": branch[0] if branch else None,
            "dirty": len(status_lines) > 0,
            "status_porcelain": status_lines,
            "changed_files": changed_files,
            "changed_file_count": len(changed_files),
            "unstaged_files": diff_files,
            "staged_files": cached_files,
            "untracked_files": untracked_files,
            "unstaged_numstat": numstat,
            "staged_numstat": cached_numstat,
            "lines_added": added + cached_added,
            "lines_deleted": deleted + cached_deleted
        },
        "hook": {
            "name": hook,
            "payload_path": payload_path,
            "payload_sha256": payload_sha256
        },
        "anti_cheat": {
            "files_changed_source": "git",
            "delta_stats_source": "git diff --numstat",
            "llm_file_accounting_allowed": False
        }
    }
}

pathlib.Path(event_path).write_text(json.dumps(event, indent=2, sort_keys=True))

with open(jsonl_path, "a") as f:
    f.write(json.dumps(event, sort_keys=True) + "\n")
PY

if [ "$HOOK" = "Stop" ]; then
  echo "\[
OCEL=.antigravity/ocel/events.jsonl
\]" >&2

  echo "\[
GitDeltaStats=OCELTail
\]" >&2

  tail -5 "$JSONL" >&2 || true
fi

exit 0
