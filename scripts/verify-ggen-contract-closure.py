#!/usr/bin/env python3
"""Run the ggen contract verifier over the complete pack ontology closure."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

from rdflib import Graph

ROOT = Path(__file__).resolve().parents[1]
CORE = ROOT / "scripts/verify-ggen-contract.py"
ONTOLOGY_DIR = ROOT / "packs/lsp-max-runtime-pack"
PARTITIONS = (ONTOLOGY_DIR / "ontology.ttl", *sorted(ONTOLOGY_DIR.glob("ontology-cmd-*.ttl")))
GENERATED_PATHS = (
    "docs/generated/GGEN_MANUFACTURING_CONTRACT.md",
    "docs/generated/STANDING_MODEL.md",
    "docs/generated/GALL_ROADMAP.md",
    "evidence/generated/verification-manifest.json",
    "tests/ggen_law_contract.rs",
    ".github/workflows/ggen-contract.yml",
    "docs/generated/CMD_PROFILE.md",
    "evidence/generated/cmd-contract.json",
    ".github/workflows/cmd-contract.yml",
)

spec = importlib.util.spec_from_file_location("lsp_max_ggen_verifier_core", CORE)
if spec is None or spec.loader is None:
    raise SystemExit("LSPMAX-VERIFIER-CORE-LOAD-FAILED")
core = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = core
spec.loader.exec_module(core)
core.GENERATED_PATHS = GENERATED_PATHS


def load_graph(path: Path = core.ONTOLOGY) -> Graph:
    graph = Graph()
    try:
        if path != core.ONTOLOGY:
            graph.parse(path, format="turtle")
        else:
            for partition in PARTITIONS:
                graph.parse(partition, format="turtle")
    except Exception as exc:
        raise core.Refusal("LSPMAX_ONTOLOGY_PARSE_REFUSED", f"{path}: {exc}") from exc
    return graph


def input_closure() -> dict[str, str]:
    paths = [
        core.CONFIG,
        core.PACK,
        *PARTITIONS,
        CORE,
        Path(__file__).resolve(),
    ]
    paths.extend(ROOT / path for path in GENERATED_PATHS)
    paths.extend(sorted(core.FIXTURES.glob("*.ttl")))
    return {
        str(path.relative_to(ROOT)): core.digest_file(path)
        for path in sorted(set(paths))
    }


core.load_graph = load_graph
core.input_closure = input_closure

if __name__ == "__main__":
    raise SystemExit(core.main())
