#!/usr/bin/env python3
"""Thin stdio/file adapter for the IO-free lsp-max CMD kernel."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from cmd_kernel import Candidate, manufacture_plan


def candidate(value: dict[str, object]) -> Candidate:
    return Candidate(
        candidate_id=str(value["candidate_id"]),
        scope=str(value["scope"]),
        signature=str(value["signature"]),
        options=tuple(str(item) for item in value["options"]),
        authority_state=str(value["authority_state"]),
        actuation_state=str(value["actuation_state"]),
        standing=str(value["standing"]),
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--request", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    payload = json.loads(args.request.read_text() if args.request else sys.stdin.read())
    plan = manufacture_plan(
        observation_digest=str(payload["observation_digest"]),
        policy_digest=str(payload["policy_digest"]),
        internal_candidate=candidate(payload["internal_candidate"]),
        external_candidate=candidate(payload["external_candidate"]),
        governed_outputs=[str(item) for item in payload["governed_outputs"]],
    )
    encoded = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded)
    else:
        sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
