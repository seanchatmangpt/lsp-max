#!/usr/bin/env python3
"""Admit the complete CMD ontology partition closure, then run the verifier core."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CORE = ROOT / "scripts/verify-cmd-profile.py"
ONTOLOGY_DIR = ROOT / "packs/lsp-max-runtime-pack"
PARTITIONS = (ONTOLOGY_DIR / "ontology.ttl", *sorted(ONTOLOGY_DIR.glob("ontology-cmd-*.ttl")))

spec = importlib.util.spec_from_file_location("lsp_max_cmd_verifier_core", CORE)
if spec is None or spec.loader is None:
    raise SystemExit("CMD-VERIFIER-CORE-LOAD-FAILED")
core = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = core
spec.loader.exec_module(core)

closure = ROOT / "target/cmd/ontology-closure.ttl"
closure.parent.mkdir(parents=True, exist_ok=True)
closure.write_bytes(b"\n".join(path.read_bytes() for path in PARTITIONS))
core.ONTOLOGY = closure


def sources() -> tuple[Path, ...]:
    config, _ = core.configs()
    paths = [
        *PARTITIONS,
        core.PACK,
        core.CONFIG,
        core.KERNEL,
        core.CLI,
        CORE,
        Path(__file__).resolve(),
        core.FIXTURES,
    ]
    paths.extend(ROOT / path for path in core.outputs(config))
    return tuple(sorted(set(paths)))


core.sources = sources

if __name__ == "__main__":
    raise SystemExit(core.main())
