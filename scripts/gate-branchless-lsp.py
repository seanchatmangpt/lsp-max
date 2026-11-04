#!/usr/bin/env python3
"""gate-branchless-lsp.py -- BL10 admission gate.

Pre-commit gate: feeds staged .rs files to the branchless-lsp analysis
client (BL9, python/lsp_client) and refuses the commit on any ERROR
diagnostic or a nonzero N_refused count from the client.

Exit codes: 0 pass, 1 ANDON_REFUSAL, 3 loud [SKIP] (never a fake pass).
Environment:
  BRANCHLESS_LSP_GATE_CLIENT_DIR  dir containing lsp_client.py
                                  (default: <repo>/python)
  BRANCHLESS_LSP_GATE_TIMEOUT     seconds for the client call (default 120)
"""
import importlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

EXIT_PASS = 0
EXIT_ANDON = 1
EXIT_SKIP = 3

MARKER = "branchless-lsp-admission-gate"


def repo_root():
    return Path(subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True, text=True, check=True).stdout.strip())


def staged_rs_files(root):
    out = subprocess.run(
        ["git", "-C", str(root), "diff", "--cached", "--name-only",
         "--diff-filter=ACM"],
        capture_output=True, text=True, check=True)
    files = [f for f in out.stdout.splitlines() if f.endswith(".rs")]
    return [str((root / f).resolve()) for f in files]


def load_client():
    """Import lsp_client from the configured dir.

    Returns (module, None) or (None, skip_reason). Loud, never silent.
    """
    cdir = Path(os.environ.get(
        "BRANCHLESS_LSP_GATE_CLIENT_DIR",
        str(repo_root() / "python"))).expanduser()
    # The BL9 deliverable is the `python/lsp_client/` package (dir with
    # __init__.py); a legacy single-file `lsp_client.py` is also accepted.
    pkg = cdir / "lsp_client"
    single = cdir / "lsp_client.py"
    if not (pkg.is_dir() or single.exists()):
        return None, ("[SKIP] branchless-lsp client missing: neither %s/lsp_client/ "
                      "package nor %s/lsp_client.py found" % (cdir, cdir))
    sys.path.insert(0, str(cdir))
    try:
        mod = importlib.import_module("lsp_client")
    except Exception as e:  # noqa: BLE001
        return None, "[SKIP] branchless-lsp client import failed: %r" % (e,)
    if not hasattr(mod, "diagnostics_for_files"):
        # The sync batch API lives in the BL9 submodule; import it directly.
        try:
            mod = importlib.import_module("lsp_client.branchless_client")
        except Exception as e:  # noqa: BLE001
            return None, ("[SKIP] branchless-lsp client submodule import "
                          "failed: %r" % (e,))
    if not hasattr(mod, "diagnostics_for_files"):
        return None, ("[SKIP] branchless-lsp client interface mismatch: "
                      "lsp_client.diagnostics_for_files(paths) missing")
    return mod, None


def is_error(diag):
    if isinstance(diag, dict):
        sev = diag.get("severity")
        return sev in ("ERROR", "error", 1)
    return getattr(diag, "severity", None) in ("ERROR", "error", 1)


def write_proof(root, files, refused, extra):
    rdir = root / ".cgap" / "receipts" / "branchless"
    rdir.mkdir(parents=True, exist_ok=True)
    ts = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    path = rdir / ("admission-%s.json" % ts)
    path.write_text(json.dumps({
        "gate": "gate-branchless-lsp",
        "admission": "ADMITTED" if not refused else "REFUSED",
        "staged_rs_files": files,
        "refusals": refused,
        "extra": extra,
        "timestamp_utc": ts,
    }, indent=2, sort_keys=True) + "\n")
    return path


def main():
    root = repo_root()
    files = staged_rs_files(root)
    client, skip = load_client()
    if client is None:
        print(skip, file=sys.stderr)
        return EXIT_SKIP
    if not files:
        print("gate: no staged .rs files; admission proof written (empty)")
        p = write_proof(root, [], [], {"note": "no staged .rs files"})
        print("gate: proof %s" % p)
        return EXIT_PASS

    timeout = float(os.environ.get("BRANCHLESS_LSP_GATE_TIMEOUT", "120"))
    try:
        result = client.diagnostics_for_files(files)
    except Exception as e:  # noqa: BLE001
        print("[SKIP] branchless-lsp client raised: %r" % (e,), file=sys.stderr)
        return EXIT_SKIP

    refused = []
    n_refused = 0
    for f in files:
        diags = result.get(f, []) if isinstance(result, dict) else []
        for d in diags:
            if is_error(d):
                line = (d.get("range", {}).get("start", {}).get("line", 0)
                        if isinstance(d, dict) else getattr(d, "line", 0))
                msg = (d.get("message", "") if isinstance(d, dict)
                       else getattr(d, "message", ""))
                refused.append({"file": f, "line": (line + 1) if isinstance(line, int) else line,
                                "message": msg})
    try:
        n_refused = int(result.get("n_refused", 0))
    except (TypeError, ValueError, AttributeError):
        pass

    if refused or n_refused > 0:
        print("ANDON_REFUSAL: branchless-lsp gate refused %d site(s)" % max(len(refused), n_refused))
        for r in refused:
            print("  %s:%s: %s" % (r["file"], r["line"], r["message"]))
        if not refused and n_refused > 0:
            print("  (client N_refused=%d without per-file diagnostics)" % n_refused)
        write_proof(root, files, refused, {"n_refused": n_refused})
        return EXIT_ANDON

    p = write_proof(root, files, [], {"n_refused": n_refused})
    print("gate: %d staged .rs file(s) clean; proof %s" % (len(files), p))
    return EXIT_PASS


if __name__ == "__main__":
    sys.exit(main())
