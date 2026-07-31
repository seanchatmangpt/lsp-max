#!/usr/bin/env python3
"""Pure combinatorial-maximalism kernel for lsp-max.

This module is deliberately IO-free. It accepts admitted data structures and
returns deterministic candidates and plans. Filesystem, process, network,
credential, and provider operations belong to adapters/brokers, never here.
"""

from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
from itertools import product
import json
from typing import Iterable, Mapping, Sequence


class KernelRefusal(ValueError):
    """Typed refusal emitted by the pure kernel."""

    def __init__(self, code: str, detail: str) -> None:
        super().__init__(f"{code}: {detail}")
        self.code = code
        self.detail = detail


@dataclass(frozen=True, order=True)
class Dimension:
    dimension_id: str
    scope: str
    options: tuple[str, ...]


@dataclass(frozen=True, order=True)
class Constraint:
    constraint_id: str
    scope: str
    when_options: frozenset[str]
    requires_options: frozenset[str]
    forbids_options: frozenset[str]
    refusal_token: str


@dataclass(frozen=True, order=True)
class Candidate:
    candidate_id: str
    scope: str
    signature: str
    options: tuple[str, ...]
    authority_state: str
    actuation_state: str
    standing: str

    def as_dict(self) -> dict[str, object]:
        return {
            "candidate_id": self.candidate_id,
            "scope": self.scope,
            "signature": self.signature,
            "options": list(self.options),
            "authority_state": self.authority_state,
            "actuation_state": self.actuation_state,
            "standing": self.standing,
        }


def canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest(value: object) -> str:
    return "sha256:" + sha256(canonical_json(value)).hexdigest()


def _validate_dimensions(dimensions: Sequence[Dimension], scope: str) -> tuple[Dimension, ...]:
    selected = tuple(sorted((d for d in dimensions if d.scope == scope), key=lambda d: d.dimension_id))
    if not selected:
        raise KernelRefusal("CMD-DIMENSION-MISSING", f"scope={scope}")
    ids = [d.dimension_id for d in selected]
    if len(ids) != len(set(ids)):
        raise KernelRefusal("CMD-DIMENSION-DUPLICATE", f"scope={scope}")
    option_ids: list[str] = []
    for dimension in selected:
        if not dimension.options:
            raise KernelRefusal("CMD-OPTION-SET-EMPTY", dimension.dimension_id)
        if len(dimension.options) != len(set(dimension.options)):
            raise KernelRefusal("CMD-OPTION-DUPLICATE", dimension.dimension_id)
        option_ids.extend(dimension.options)
    if len(option_ids) != len(set(option_ids)):
        raise KernelRefusal("CMD-OPTION-IDENTITY-COLLISION", f"scope={scope}")
    return selected


def _constraint_satisfied(options: frozenset[str], constraint: Constraint) -> bool:
    if not constraint.when_options.issubset(options):
        return True
    return constraint.requires_options.issubset(options) and options.isdisjoint(constraint.forbids_options)


def enumerate_candidates(
    dimensions: Sequence[Dimension],
    constraints: Sequence[Constraint],
    scope: str,
    candidate_ceiling: int,
) -> tuple[Candidate, ...]:
    """Enumerate the complete bounded product, then prune by admitted constraints."""

    selected = _validate_dimensions(dimensions, scope)
    raw_count = 1
    for dimension in selected:
        raw_count *= len(dimension.options)
    if raw_count > candidate_ceiling:
        raise KernelRefusal(
            "CMD-COVERAGE-OVERFLOW",
            f"scope={scope} raw={raw_count} ceiling={candidate_ceiling}",
        )

    scoped_constraints = tuple(sorted((c for c in constraints if c.scope == scope), key=lambda c: c.constraint_id))
    candidates: list[Candidate] = []
    signatures: set[str] = set()
    for combination in product(*(dimension.options for dimension in selected)):
        option_set = frozenset(combination)
        if not all(_constraint_satisfied(option_set, constraint) for constraint in scoped_constraints):
            continue
        ordered_options = tuple(combination)
        signature = digest({"scope": scope, "options": ordered_options})
        if signature in signatures:
            raise KernelRefusal("CMD-CANDIDATE-DUPLICATE", signature)
        signatures.add(signature)
        external_mutation = "consequence:external-mutation" in option_set
        authority_state = "UNAUTHORIZED" if external_mutation else "NOT_REQUIRED"
        actuation_state = "INERT_INTENT_ONLY" if external_mutation else "NOT_ACTUATED"
        candidate_id = f"candidate:{scope}:{signature.removeprefix('sha256:')[:24]}"
        candidates.append(
            Candidate(
                candidate_id=candidate_id,
                scope=scope,
                signature=signature,
                options=ordered_options,
                authority_state=authority_state,
                actuation_state=actuation_state,
                standing="UNKNOWN",
            )
        )
    if not candidates:
        raise KernelRefusal("CMD-COVERAGE-UNDERFLOW", f"scope={scope}")
    return tuple(candidates)


def valid_pair_coverage(candidates: Sequence[Candidate]) -> dict[str, object]:
    """Recompute all cross-dimension option pairs represented by valid candidates."""

    pairs: set[tuple[str, str]] = set()
    for candidate in candidates:
        options = candidate.options
        for left_index in range(len(options)):
            for right_index in range(left_index + 1, len(options)):
                pairs.add((options[left_index], options[right_index]))
    ordered = sorted(pairs)
    return {
        "covered_pairs": len(ordered),
        "pair_digest": digest(ordered),
    }


def manufacture_plan(
    *,
    observation_digest: str,
    policy_digest: str,
    internal_candidate: Candidate,
    external_candidate: Candidate,
    governed_outputs: Sequence[str],
) -> dict[str, object]:
    """Manufacture an inert deterministic plan. This function never actuates."""

    if internal_candidate.scope != "internal" or external_candidate.scope != "external":
        raise KernelRefusal("CMD-CANDIDATE-SCOPE", "plan requires internal+external candidates")
    outputs = tuple(sorted(governed_outputs))
    if len(outputs) != len(set(outputs)):
        raise KernelRefusal("OWN-MULTIPLE-EXCLUSIVE", "duplicate governed output")
    plan_body: dict[str, object] = {
        "schema": "chatmangpt.cmd.plan.v1",
        "observation_digest": observation_digest,
        "policy_digest": policy_digest,
        "internal_candidate": internal_candidate.as_dict(),
        "external_candidate": external_candidate.as_dict(),
        "ownership": {"exclusive": list(outputs), "shared_merge": []},
        "consequences": {
            "local": [f"stage:{path}" for path in outputs],
            "external_intents": (
                ["intent:brce-external-consequence"]
                if external_candidate.actuation_state == "INERT_INTENT_ONLY"
                else []
            ),
        },
        "standing": "UNKNOWN",
        "actuation_performed": False,
    }
    plan_body["plan_digest"] = digest(plan_body)
    return plan_body


def reorder_dimensions(dimensions: Iterable[Dimension]) -> tuple[Dimension, ...]:
    """Test helper: canonicalize arbitrary input order."""

    return tuple(sorted(dimensions, key=lambda d: (d.scope, d.dimension_id)))
