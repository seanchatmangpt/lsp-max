import json
import hashlib
import subprocess
import sys

def generate_baseline(repo_path):
    allowed_ignored_directories = [
        ".agents",
        ".claude",
        ".ggen",
        "target",
        "wasm4pm/target",
        "vendors",
        ".wasm4pm",
        ".wasm4pm/sessions",
        ".wasm4pm/compaction-checkpoints"
    ]
    forbidden_generated_paths = []
    ignored_inventory = [
        ".DS_Store",
        ".gc-sealed-baseline",
        "Justfile",
        "package-lock.json",
        "pnpm-lock.yaml",
        "node_modules",
        "examples/node_modules"
    ]

    res = subprocess.run(["git", "status", "--porcelain", "--ignored"], cwd=repo_path, capture_output=True, text=True)
    tracked_status = {}
    for line in res.stdout.splitlines():
        if not line:
            continue
        status = line[:2]
        path = line[3:]
        if path.startswith('"') and path.endswith('"'):
            path = path[1:-1]
        
        # Trim leading/trailing spaces from status
        status_clean = status.strip()
        if status_clean in ["??", "!!"]:
            continue
        # It is a tracked modification
        tracked_status[path] = status_clean

    sorted_tracked = {k: tracked_status[k] for k in sorted(tracked_status.keys())}

    # Construct keys in the exact order of Rust BaselineManifest struct
    data = {
        "allowed_ignored_directories": allowed_ignored_directories,
        "forbidden_generated_paths": forbidden_generated_paths,
        "ignored_inventory": ignored_inventory,
        "tracked_status": sorted_tracked
    }

    serialized = json.dumps(data, separators=(',', ':'))
    digest = hashlib.sha256(serialized.encode('utf-8')).hexdigest()

    data["digest"] = digest
    out_path = f"{repo_path}/.gc-sealed-baseline"
    with open(out_path, "w") as f:
        json.dump(data, f, indent=2)
    print(f"Generated {out_path} with digest {digest}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python generate_baseline.py <repo_path>")
        sys.exit(1)
    generate_baseline(sys.argv[1])
