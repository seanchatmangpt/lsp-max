#!/usr/bin/env python3
"""Execute one independently addressable CMD verifier suite."""
from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
CLOSURE = ROOT / "scripts/verify-cmd-profile-closure.py"
spec = importlib.util.spec_from_file_location("lsp_max_cmd_closure", CLOSURE)
if spec is None or spec.loader is None:
    raise SystemExit("CMD-CLOSURE-LOAD-FAILED")
closure = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = closure
spec.loader.exec_module(closure)
core = closure.core


def context(out: Path):
    config, pack = core.configs()
    model = core.model()
    internal, external, coverage, _ = core.coverage(model)
    observation, authority = core.observe(config)
    policy = core.digest(
        {
            "candidate_ceiling": model.candidate_ceiling,
            "output_ceiling": model.output_ceiling,
        }
    )
    external_candidate = next(
        (item for item in external if item.actuation_state == "INERT_INTENT_ONLY"),
        external[0],
    )
    plan = core.manufacture_plan(
        observation_digest=core.digest(
            {"revision": observation["revision"], "tree": observation["tree_digest"]}
        ),
        policy_digest=policy,
        internal_candidate=internal[0],
        external_candidate=external_candidate,
        governed_outputs=core.outputs(config),
    )
    request = {
        "observation_digest": plan["observation_digest"],
        "policy_digest": plan["policy_digest"],
        "internal_candidate": internal[0].as_dict(),
        "external_candidate": external_candidate.as_dict(),
        "governed_outputs": list(core.outputs(config)),
    }
    return (
        config,
        pack,
        model,
        internal,
        external,
        coverage,
        observation,
        authority,
        plan,
        request,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "suite",
        choices=[
            "protocol-unit",
            "property-fuzz",
            "stdio-http-integration",
            "cli-e2e",
            "security",
            "chaos",
            "stress",
            "benchmark",
        ],
    )
    parser.add_argument("--output", type=Path, default=ROOT / "target/cmd-suite")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    (
        config,
        pack,
        model,
        _internal,
        _external,
        coverage,
        observation,
        authority,
        plan,
        request,
    ) = context(args.output)

    if args.suite == "protocol-unit":
        detail = core.protocol(config, pack, model)
    elif args.suite == "property-fuzz":
        detail = coverage
    elif args.suite == "stdio-http-integration":
        detail = {
            "observation_files": observation["file_count"],
            "http": core.http_check(request),
        }
    elif args.suite == "cli-e2e":
        value = core.cli_check(request, args.output)
        if value["plan_digest"] != plan["plan_digest"]:
            raise core.Refusal("CMD-CLI-DIVERGENCE", "plan digest")
        detail = {"plan_digest": value["plan_digest"]}
    elif args.suite == "security":
        detail = core.negatives(observation, authority, model)
    elif args.suite == "chaos":
        detail = core.chaos()
    elif args.suite == "stress":
        started = time.perf_counter()
        counts = [
            len(
                core.enumerate_candidates(
                    model.dimensions,
                    model.constraints,
                    "external",
                    model.candidate_ceiling,
                )
            )
            for _ in range(10)
        ]
        elapsed = time.perf_counter() - started
        if len(set(counts)) != 1:
            raise core.Refusal("CMD-STRESS-DIVERGENCE", str(counts))
        detail = {
            "iterations": 10,
            "candidate_count": counts[0],
            "seconds": elapsed,
        }
    else:
        started = time.perf_counter()
        core.coverage(model)
        elapsed = time.perf_counter() - started
        if elapsed > model.benchmark_seconds:
            raise core.Refusal("CMD-BENCHMARK-BUDGET", str(elapsed))
        detail = {
            "coverage_seconds": elapsed,
            "budget_seconds": model.benchmark_seconds,
        }

    path = args.output / f"{args.suite}.json"
    core.write(
        path,
        {"suite": args.suite, "state": "PARTIAL_ALIVE", "detail": detail},
    )
    print(f"PARTIAL_ALIVE {args.suite}: {json.dumps(detail, sort_keys=True)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
