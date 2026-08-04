#!/usr/bin/env python3
"""Generate and verify the LSP 3.18 / LSIF 0.6.0 protocol court ledger.

The ledger is a projection of canonical repository sources.  It records gaps;
it never manufactures execution witnesses or receipts.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "protocol-court.json"
ONTOLOGY = ROOT / "ontology/lsp318.ttl"
LSIF_COVERAGE = ROOT / "crates/lsp-max-lsif/src/coverage.rs"
STATUSES = {"ADMITTED", "BLOCKED", "CANDIDATE", "OPEN", "PARTIAL", "REFUSED", "UNKNOWN"}


def digest(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def section(text: str, constant: str) -> str:
    match = re.search(
        rf"pub const {constant}:.*?=\s*&\[(.*?)\n\];", text, flags=re.DOTALL
    )
    if not match:
        raise ValueError(f"REFUSED: cannot locate {constant}")
    return match.group(1)


def quoted_labels(body: str) -> list[str]:
    return re.findall(r'^\s*"([^"]+)",?\s*$', body, flags=re.MULTILINE)


def consumer_map(body: str) -> dict[str, tuple[str, str | None]]:
    starts = list(
        re.finditer(r'^ {4}\(\s*"([^"]+)"\s*,', body, flags=re.MULTILINE)
    )
    result: dict[str, tuple[str, str | None]] = {}
    for index, match in enumerate(starts):
        label = match.group(1)
        end = starts[index + 1].start() if index + 1 < len(starts) else len(body)
        record = body[match.start() : end]
        admitted = re.search(r'ConsumerStatus::Admitted\(\s*"([^"]+)"', record)
        if admitted:
            result[label] = ("ADMITTED", admitted.group(1))
        elif "ConsumerStatus::OpenSubstrate" in record:
            result[label] = ("OPEN", None)
        else:
            raise ValueError(f"REFUSED: no consumer status for LSIF label {label}")
    return result


def lsp_entries() -> list[dict[str, object]]:
    text = ONTOLOGY.read_text()
    declared = re.findall(r'lsp:methodName\s+"([^"]+)"', text)
    blocks = re.findall(
        r'lsp:[^\s]+\s+a\s+lsp:(Request|Notification|MaxExtension)\s*;(.*?)(?=\n\s*lsp:[^\s]+\s+a\s+lsp:|\Z)',
        text,
        flags=re.DOTALL,
    )
    entries = []
    for kind, body in blocks:
        method = re.search(r'lsp:methodName\s+"([^"]+)"', body)
        status = re.search(r'law:status\s+law:(\w+)', body)
        if not method:
            continue
        if not status or status.group(1) not in STATUSES:
            raise ValueError(f"REFUSED: invalid law status for {method.group(1)}")
        name = method.group(1)
        entries.append(
            {
                "protocol": "LSP 3.18",
                "kind": kind.lower(),
                "name": name,
                "status": status.group(1),
                "emittable": None,
                "positive_witness": None,
                "falsifier": f"protocol-court:lsp:{name}:missing-behavioral-matrix",
                "consumer": None,
                "receipt": None,
                "replay": "python3 scripts/protocol-court.py --check",
            }
        )
    observed = [str(entry["name"]) for entry in entries]
    if len(declared) != len(set(declared)) or set(declared) != set(observed):
        raise ValueError("REFUSED: LSP ontology declarations were not projected exactly once")
    return sorted(entries, key=lambda item: str(item["name"]))


def lsif_entries() -> list[dict[str, object]]:
    text = LSIF_COVERAGE.read_text()
    entries = []
    for kind, spec_name, builder_name in (
        ("vertex", "SPEC_VERTEX_LABELS", "BUILDER_VERTEX_LABELS"),
        ("edge", "SPEC_EDGE_LABELS", "BUILDER_EDGE_LABELS"),
    ):
        labels = quoted_labels(section(text, spec_name))
        consumers = consumer_map(section(text, builder_name))
        if len(labels) != len(set(labels)) or not set(consumers).issubset(labels):
            raise ValueError(f"REFUSED: inconsistent {kind} vocabulary")
        for label in labels:
            consumer_status, witness = consumers.get(label, ("UNKNOWN", None))
            entries.append(
                {
                    "protocol": "LSIF 0.6.0",
                    "kind": kind,
                    "name": label,
                    "status": consumer_status,
                    "emittable": label in consumers,
                    "positive_witness": witness,
                    "falsifier": None
                    if witness
                    else f"protocol-court:lsif:{kind}:{label}:missing-consumer-witness",
                    "consumer": "lsp-max-lsif indexer" if witness else None,
                    "receipt": None,
                    "replay": "cargo test -p lsp-max-lsif",
                }
            )
    return sorted(entries, key=lambda item: (str(item["kind"]), str(item["name"])))


def build() -> dict[str, object]:
    entries = lsp_entries() + lsif_entries()
    identities = [(entry["protocol"], entry["kind"], entry["name"]) for entry in entries]
    if len(identities) != len(set(identities)):
        raise ValueError("REFUSED: duplicate protocol court identity")

    gaps = {
        "lsp_not_admitted": sum(
            entry["protocol"] == "LSP 3.18" and entry["status"] != "ADMITTED"
            for entry in entries
        ),
        "lsif_open_or_unknown": sum(
            entry["protocol"] == "LSIF 0.6.0"
            and entry["status"] in {"OPEN", "UNKNOWN"}
            for entry in entries
        ),
        "missing_positive_witness": sum(entry["positive_witness"] is None for entry in entries),
        "missing_receipt": sum(entry["receipt"] is None for entry in entries),
    }
    return {
        "schema_version": 1,
        "standing": "PARTIAL_ALIVE" if any(gaps.values()) else "ALIVE",
        "crown": not any(gaps.values()),
        "sources": {
            "lsp_ontology": {"path": str(ONTOLOGY.relative_to(ROOT)), "digest": digest(ONTOLOGY)},
            "lsif_coverage": {
                "path": str(LSIF_COVERAGE.relative_to(ROOT)),
                "digest": digest(LSIF_COVERAGE),
            },
        },
        "required_fields": [
            "protocol",
            "kind",
            "name",
            "status",
            "emittable",
            "positive_witness",
            "falsifier",
            "consumer",
            "receipt",
            "replay",
        ],
        "gaps": gaps,
        "entries": entries,
    }


def encoded() -> bytes:
    return (json.dumps(build(), indent=2, sort_keys=True) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="refuse stale or malformed ledger")
    args = parser.parse_args()
    expected = encoded()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != expected:
            print("REFUSED: protocol-court.json is stale; regenerate it", file=sys.stderr)
            return 1
        ledger = json.loads(expected)
        print(
            f"{ledger['standing']}: entries={len(ledger['entries'])} "
            f"gaps={sum(ledger['gaps'].values())} crown={str(ledger['crown']).lower()}"
        )
        return 0
    OUTPUT.write_bytes(expected)
    print(f"wrote {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
