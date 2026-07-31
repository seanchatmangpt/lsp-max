#!/usr/bin/env python3
"""Independent verifier for the lsp-max ggen manufacturing contract.

This verifier does not establish runtime standing by inspection. It admits the
semantic contract, executes named negative fixtures, and emits a BLAKE3-bound
receipt only after all observed checks pass. Replay recomputes the exact input
closure and check result set; the receipt never contains its own hash.
"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

try:
    from rdflib import Graph, Literal, Namespace, RDF
except ImportError as exc:
    print(
        "LSPMAX_VERIFIER_DEPENDENCY_MISSING: install rdflib==7.1.4",
        file=sys.stderr,
    )
    raise SystemExit(78) from exc

ROOT = Path(__file__).resolve().parents[1]
ONTOLOGY = ROOT / "packs/lsp-max-runtime-pack/ontology.ttl"
CONFIG = ROOT / "ggen.toml"
PACK = ROOT / "packs/lsp-max-runtime-pack/pack.toml"
FIXTURES = ROOT / "tests/fixtures/ggen"

LSP = Namespace("https://chatmangpt.com/ns/lsp-max#")
SKOS = Namespace("http://www.w3.org/2004/02/skos/core#")

GENERATED_PATHS = (
    "docs/generated/GGEN_MANUFACTURING_CONTRACT.md",
    "docs/generated/STANDING_MODEL.md",
    "docs/generated/GALL_ROADMAP.md",
    "evidence/generated/verification-manifest.json",
    "tests/ggen_law_contract.rs",
    ".github/workflows/ggen-contract.yml",
)

REQUIRED_EVIDENCE = {
    "EVIDENCE-WITNESS",
    "EVIDENCE-FALSIFIER",
    "EVIDENCE-INDEPENDENT-VERIFIER",
    "EVIDENCE-RECEIPT-VERIFIER",
    "EVIDENCE-REPLAY",
}


class Refusal(RuntimeError):
    def __init__(self, code: str, detail: str) -> None:
        super().__init__(f"{code}: {detail}")
        self.code = code
        self.detail = detail


@dataclass(frozen=True)
class CheckResult:
    name: str
    state: str
    detail: str

    def as_dict(self) -> dict[str, str]:
        return {"name": self.name, "state": self.state, "detail": self.detail}


def one(values: Iterable[object], code: str, detail: str) -> object:
    materialized = list(values)
    if len(materialized) != 1:
        raise Refusal(code, f"{detail}; observed={len(materialized)}")
    return materialized[0]


def text(graph: Graph, subject: object, predicate: object, code: str) -> str:
    value = one(graph.objects(subject, predicate), code, f"expected one {predicate}")
    if not isinstance(value, Literal):
        raise Refusal(code, f"expected literal for {predicate}")
    return str(value)


def load_graph(path: Path = ONTOLOGY) -> Graph:
    graph = Graph()
    try:
        graph.parse(path, format="turtle")
    except Exception as exc:
        raise Refusal("LSPMAX_ONTOLOGY_PARSE_REFUSED", f"{path}: {exc}") from exc
    return graph


def validate_config() -> CheckResult:
    with CONFIG.open("rb") as stream:
        config = tomllib.load(stream)
    with PACK.open("rb") as stream:
        pack = tomllib.load(stream)

    if config["ontology"]["source"] != "packs/lsp-max-runtime-pack/ontology.ttl":
        raise Refusal(
            "LSPMAX_ONTOLOGY_AUTHORITY_REFUSED",
            "ggen.toml source is not the canonical pack graph",
        )
    if pack["pack"]["name"] != "lsp-max-runtime-pack":
        raise Refusal("LSPMAX_PACK_IDENTITY_REFUSED", "unexpected pack identity")

    rules = config.get("generation", {}).get("rules", [])
    outputs = [rule["output_file"] for rule in rules]
    if len(outputs) != len(set(outputs)):
        raise Refusal(
            "LSPMAX_MULTIPLE_TEMPLATE_OWNERS_REFUSED",
            "duplicate generated output owner",
        )
    if tuple(outputs) != GENERATED_PATHS:
        raise Refusal(
            "LSPMAX_GENERATED_CLOSURE_REFUSED",
            f"declared outputs differ from governed closure: {outputs}",
        )
    return CheckResult(
        "config-and-pack",
        "PARTIAL_ALIVE",
        f"rules={len(rules)} unique_outputs={len(outputs)}",
    )


def validate_graph(graph: Graph) -> list[CheckResult]:
    programs = list(graph.subjects(RDF.type, LSP.Program))
    program = one(
        programs,
        "LSPMAX_PROGRAM_CARDINALITY_REFUSED",
        "expected one lsp:Program",
    )
    if text(graph, program, LSP.requiredBroker, "LSPMAX_BROKER_MISSING") != "BRCE":
        raise Refusal("LSPMAX_DIRECT_ACTUATION_REFUSED", "program broker is not BRCE")
    if text(graph, program, LSP.standing, "LSPMAX_STANDING_MISSING") != "UNKNOWN":
        raise Refusal(
            "LSPMAX_STANDING_OVERCLAIM_REFUSED",
            "new manufacturing program must begin UNKNOWN",
        )

    blocks = sorted(graph.subjects(RDF.type, LSP.BuildingBlock), key=str)
    if len(blocks) < 7:
        raise Refusal(
            "LSPMAX_BUILDING_BLOCK_CLOSURE_REFUSED",
            f"expected >=7 blocks, observed={len(blocks)}",
        )
    for block in blocks:
        for predicate in (
            LSP.blockId,
            LSP.architectureFacet,
            LSP.realization,
            LSP.lifecycle,
            LSP.standing,
            LSP.requiredBroker,
        ):
            text(
                graph,
                block,
                predicate,
                "LSPMAX_BUILDING_BLOCK_CONTRACT_REFUSED",
            )
        if text(graph, block, LSP.requiredBroker, "LSPMAX_BROKER_MISSING") != "BRCE":
            raise Refusal("LSPMAX_DIRECT_ACTUATION_REFUSED", f"{block} bypasses BRCE")

    obligations = {
        str(value)
        for subject in graph.subjects(RDF.type, LSP.EvidenceObligation)
        for value in graph.objects(subject, LSP.obligationId)
    }
    if obligations != REQUIRED_EVIDENCE:
        raise Refusal(
            "LSPMAX_ALIVE_EVIDENCE_INCOMPLETE",
            f"evidence closure={sorted(obligations)}",
        )

    laws = list(graph.subjects(RDF.type, LSP.Law))
    tokens: list[str] = []
    for law in laws:
        text(graph, law, LSP.lawId, "LSPMAX_LAW_ID_MISSING")
        text(graph, law, SKOS.prefLabel, "LSPMAX_LAW_LABEL_MISSING")
        text(graph, law, LSP.statement, "LSPMAX_LAW_STATEMENT_MISSING")
        tokens.append(
            text(graph, law, LSP.refusalToken, "LSPMAX_REFUSAL_TOKEN_MISSING")
        )
    if len(tokens) != len(set(tokens)):
        raise Refusal(
            "LSPMAX_REFUSAL_TOKEN_COLLISION",
            "law refusal tokens are not unique",
        )

    checkpoints = []
    for checkpoint in graph.subjects(RDF.type, LSP.GallCheckpoint):
        checkpoints.append(
            (
                int(text(graph, checkpoint, LSP.order, "LSPMAX_GALL_ORDER_MISSING")),
                text(graph, checkpoint, LSP.checkpointId, "LSPMAX_GALL_ID_MISSING"),
            )
        )
    checkpoints.sort()
    expected = [(index, f"GALL-{index:03d}") for index in range(1, 11)]
    if checkpoints != expected:
        raise Refusal("LSPMAX_GALL_CLOSURE_REFUSED", f"observed={checkpoints}")

    return [
        CheckResult(
            "program-admission",
            "PARTIAL_ALIVE",
            "program=1 broker=BRCE standing=UNKNOWN",
        ),
        CheckResult(
            "building-block-contracts",
            "PARTIAL_ALIVE",
            f"blocks={len(blocks)}",
        ),
        CheckResult(
            "alive-evidence-closure",
            "PARTIAL_ALIVE",
            f"obligations={len(obligations)}",
        ),
        CheckResult(
            "typed-refusal-law",
            "PARTIAL_ALIVE",
            f"laws={len(laws)} tokens={len(tokens)}",
        ),
        CheckResult(
            "gall-total-order",
            "PARTIAL_ALIVE",
            "checkpoints=10 crown=GALL-010",
        ),
    ]


def validate_generated_surfaces() -> CheckResult:
    for relative in GENERATED_PATHS:
        path = ROOT / relative
        if not path.is_file():
            raise Refusal("LSPMAX_GENERATED_SURFACE_MISSING", relative)
        content = path.read_text(encoding="utf-8")
        if "generated" not in content.lower():
            raise Refusal("LSPMAX_GENERATED_PROVENANCE_MISSING", relative)

    manifest = json.loads(
        (ROOT / "evidence/generated/verification-manifest.json").read_text()
    )
    if manifest["standing"] != "UNKNOWN" or manifest["required_broker"] != "BRCE":
        raise Refusal(
            "LSPMAX_GENERATED_MANIFEST_OVERCLAIM_REFUSED",
            str(manifest),
        )
    if set(manifest["evidence_obligations"]) != REQUIRED_EVIDENCE:
        raise Refusal(
            "LSPMAX_GENERATED_MANIFEST_EVIDENCE_REFUSED",
            str(manifest["evidence_obligations"]),
        )
    return CheckResult(
        "generated-surfaces",
        "PARTIAL_ALIVE",
        f"surfaces={len(GENERATED_PATHS)}",
    )


def validate_fixture(path: Path) -> str:
    graph = load_graph(path)
    subject = one(
        graph.subjects(RDF.type, LSP.BuildingBlock),
        "LSPMAX_FIXTURE_SHAPE_REFUSED",
        path.name,
    )
    direct_values = list(graph.objects(subject, LSP.directActuation))
    if direct_values and any(
        bool(value.toPython()) for value in direct_values if isinstance(value, Literal)
    ):
        raise Refusal("LSPMAX_DIRECT_ACTUATION_REFUSED", path.name)
    standing_values = [str(value) for value in graph.objects(subject, LSP.standing)]
    if "ALIVE" in standing_values:
        evidence = {str(value) for value in graph.objects(subject, LSP.evidence)}
        if evidence != REQUIRED_EVIDENCE:
            raise Refusal("LSPMAX_ALIVE_EVIDENCE_INCOMPLETE", path.name)
    return "ADMITTED"


def execute_negative_fixtures() -> CheckResult:
    expected = {
        "direct-actuation.ttl": "LSPMAX_DIRECT_ACTUATION_REFUSED",
        "alive-without-replay.ttl": "LSPMAX_ALIVE_EVIDENCE_INCOMPLETE",
    }
    observed: dict[str, str] = {}
    for name, code in expected.items():
        try:
            validate_fixture(FIXTURES / name)
        except Refusal as refusal:
            observed[name] = refusal.code
            if refusal.code != code:
                raise Refusal(
                    "LSPMAX_NEGATIVE_FIXTURE_WRONG_REFUSAL",
                    f"{name}: {refusal.code} != {code}",
                )
        else:
            raise Refusal("LSPMAX_NEGATIVE_FIXTURE_ADMITTED", name)
    return CheckResult(
        "negative-fixtures",
        "PARTIAL_ALIVE",
        json.dumps(observed, sort_keys=True),
    )


def blake3_hex(payload: bytes) -> str:
    try:
        from blake3 import blake3
    except ImportError as exc:
        raise Refusal(
            "LSPMAX_BLAKE3_DEPENDENCY_MISSING",
            "receipt/replay requires blake3==1.0.5",
        ) from exc
    return blake3(payload).hexdigest()


def digest_file(path: Path) -> str:
    return blake3_hex(path.read_bytes())


def input_closure() -> dict[str, str]:
    paths = [CONFIG, PACK, ONTOLOGY, Path(__file__).resolve()]
    paths.extend(ROOT / path for path in GENERATED_PATHS)
    paths.extend(sorted(FIXTURES.glob("*.ttl")))
    return {
        str(path.relative_to(ROOT)): digest_file(path)
        for path in sorted(paths)
    }


def closure_root(inputs: dict[str, str]) -> str:
    payload = "".join(
        f"{path}\0{digest}\n" for path, digest in sorted(inputs.items())
    ).encode()
    return blake3_hex(payload)


def execute(check: bool, negative: bool) -> list[CheckResult]:
    results: list[CheckResult] = []
    if check:
        results.append(validate_config())
        graph = load_graph()
        results.extend(validate_graph(graph))
        results.append(validate_generated_surfaces())
    if negative:
        results.append(execute_negative_fixtures())
    return results


def emit_receipt(path: Path, results: list[CheckResult]) -> None:
    if not results:
        raise Refusal("LSPMAX_EMPTY_RECEIPT_REFUSED", "no observed checks")
    inputs = input_closure()
    receipt = {
        "schema": "chatmangpt.lsp-max.execution-receipt.v1",
        "algorithm": "BLAKE3",
        "standing": "PARTIAL_ALIVE",
        "claim_boundary": "ggen semantic contract and generated verification surfaces only",
        "input_root": closure_root(inputs),
        "inputs": inputs,
        "checks": [result.as_dict() for result in results],
        "exclusions": [
            "workspace-wide tests are not claimed by this receipt",
            "runtime LSP behavior is not claimed by this receipt",
            "ALIVE crown standing is not claimed by this receipt",
        ],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        f"LSPMAX_RECEIPT_EMITTED path={path} input_root={receipt['input_root']}"
    )


def replay(path: Path) -> None:
    receipt = json.loads(path.read_text(encoding="utf-8"))
    if receipt.get("schema") != "chatmangpt.lsp-max.execution-receipt.v1":
        raise Refusal(
            "LSPMAX_RECEIPT_SCHEMA_REFUSED",
            str(receipt.get("schema")),
        )
    current = input_closure()
    current_root = closure_root(current)
    if current != receipt.get("inputs") or current_root != receipt.get("input_root"):
        raise Refusal(
            "LSPMAX_REPLAY_INPUT_DRIFT_REFUSED",
            f"expected={receipt.get('input_root')} observed={current_root}",
        )
    results = execute(check=True, negative=True)
    if [result.as_dict() for result in results] != receipt.get("checks"):
        raise Refusal(
            "LSPMAX_REPLAY_RESULT_DRIFT_REFUSED",
            "check result set differs",
        )
    print(f"LSPMAX_REPLAY_VERIFIED input_root={current_root} checks={len(results)}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--negative-fixtures", action="store_true")
    parser.add_argument("--emit-receipt", type=Path)
    parser.add_argument("--replay", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.replay:
            replay(args.replay)
            return 0
        if not args.check and not args.negative_fixtures:
            args.check = True
            args.negative_fixtures = True
        results = execute(args.check, args.negative_fixtures)
        for result in results:
            print(f"{result.state} {result.name}: {result.detail}")
        if args.emit_receipt:
            emit_receipt(args.emit_receipt, results)
        return 0
    except Refusal as refusal:
        print(f"{refusal.code}: {refusal.detail}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
