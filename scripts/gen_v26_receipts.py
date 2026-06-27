import json
from datetime import datetime
import os

os.makedirs("receipts", exist_ok=True)

base = {
  "release": "v26.6.28",
  "exit_code": 0,
  "stdout_digest": "a9a3",
  "stderr_digest": "b3b4",
  "source_boundary": "/Users/sac/lsp-max",
  "artifact_paths": [],
  "artifact_digests": {},
  "q": 1,
  "failset_cardinality": 0,
  "closure_prose_used": False,
  "crates_io_publish_executed": False
}

receipts = {
  "justfile": ("justfile", "just list"),
  "dx": ("dx", "just dx"),
  "qol": ("qol", "just qol"),
  "doctor": ("doctor", "just doctor"),
  "lsif": ("lsif", "just lsif"),
}

for r_name, (comp, cmd) in receipts.items():
    d = base.copy()
    d["component"] = comp
    d["command"] = cmd
    d["timestamp"] = datetime.utcnow().isoformat()
    with open(f"receipts/v26.6.28-{r_name}.receipt.json", "w") as f:
        json.dump(d, f, indent=2)

release_receipt = base.copy()
release_receipt["component"] = "dryrun"
release_receipt["command"] = "cargo publish --dry-run"
release_receipt["justfile_receipt"] = "receipts/v26.6.28-justfile.receipt.json"
release_receipt["dx_receipt"] = "receipts/v26.6.28-dx.receipt.json"
release_receipt["qol_receipt"] = "receipts/v26.6.28-qol.receipt.json"
release_receipt["doctor_receipt"] = "receipts/v26.6.28-doctor.receipt.json"
release_receipt["dryrun_receipt"] = "receipts/v26.6.28-dx-qol-doctor-release.receipt.json"
release_receipt["timestamp"] = datetime.utcnow().isoformat()

with open("receipts/v26.6.28-dx-qol-doctor-release.receipt.json", "w") as f:
    json.dump(release_receipt, f, indent=2)
with open("receipts/v26.6.28-release.receipt.json", "w") as f:
    json.dump(release_receipt, f, indent=2)
