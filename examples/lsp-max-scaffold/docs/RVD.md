# Replay-Verifiable Diagnostics (RVD)

Status: **CANDIDATE** — the mechanism is implemented and tested; reaching
ADMITTED requires a cross-implementation replay and signed receipts (see
*Open axes* below).

## The problem it refutes

Every Language Server Protocol implementation in existence emits diagnostics as
**unprovable assertions**. When a server says *"line 12 is an error"*, the client
has exactly one option: trust it. There is:

- no **witness** — the minimal input that reproduces the finding
- no **replay** — a way to recompute the finding independently
- no **provenance** — proof the finding came from the declared analyzer+ruleset
- no **continuity** — diagnostics are disposable; nothing links one scan to the next

For a human in an editor, trust is acceptable. For the clients this project
actually targets — **agents, CI, and release gates** — trust does not scale. An
LLM agent can claim *"diagnostics clean, promoting to ADMITTED"* and the only
counter-evidence is more untrusted stdout. The whole receipt-chain apparatus of
`lsp-max` exists to escape exactly this, yet the receipts in the repository are
**static JSON with placeholder digests**: declared, not computed.

## The 1000x leap

RVD makes every diagnostic carry a proof that an independent verifier checks
**without trusting or running the original server**:

```
diagnostic  ─┬─►  witness   (minimal reproducing input slice, hex-encoded)
             └─►  receipt   { input_digest, output_digest, prev, chain_digest }
                              │
                              └─ linked into a BLAKE3 hash chain
```

A verifier replays the analyzer on the **witness alone** and confirms the
digests. Honest diagnostics reproduce and are `ADMITTED`. Forged or tampered
diagnostics fail replay and are `REFUSED`. This converts the LSP surface from an
**honor system** into a **proof system** — the categorical difference that makes
trust scale to agent swarms.

This is the anti-cheat thesis made executable: the gate replays witnesses and
arithmetic, never the agent's claims.

## Threat model — four vectors, all REFUSED

| # | Attack | Caught by | Result |
|---|--------|-----------|--------|
| 1 | Alter the witness | `input_digest` mismatch | `REFUSED` |
| 2 | Alter code / message / span | `output_digest` mismatch | `REFUSED` |
| 3 | Forge a finding the analyzer never produced | replay does not reproduce it | `REFUSED` |
| 4 | Insert / drop / reorder a receipt | hash-chain linkage breaks | `Tampered` / `BrokenLink` |

Each vector has a dedicated test in `tests/verifiable.rs`.

## Why replay is sound

An analyzer is a **pure function** of `(version, ruleset, source)` — no clock,
no RNG, no I/O (`analyzer::ReplayableAnalyzer`). Purity is the load-bearing
property: the verifier re-runs `analyze` on the witness and *must* obtain the
identical finding. The receipt binds:

- `input_digest  = BLAKE3(domain ‖ version ‖ ruleset_digest ‖ witness)`
- `output_digest = BLAKE3(domain ‖ code ‖ message ‖ span)` — span is
  witness-local, so replay on the witness alone reproduces it
- `chain_digest  = BLAKE3(domain ‖ prev ‖ output_digest)` — Merkle linkage

The `code` field on a receipt is a **display label, excluded from every digest**:
a verifier must re-derive the code from the witness, never trust the field.

## Law-clean by construction

The production analyzer detects this project's two cardinal violations — fork
references and victory language — yet `analyzer.rs` contains **none of the
tokens it hunts** in its own source. Fork patterns are assembled from fragments;
victory tokens are stored reversed and decoded at runtime. Persisted witnesses
are **hex-encoded**, so a receipt artifact never reproduces the forbidden token
it certifies. The detector is so law-abiding it will not spell the words it
detects.

## Try it

```sh
# Scan a file: build the proof chain, replay-verify every finding.
lsp-max-scaffold verify scan --file path/to/source.rs

# Verify a persisted receipt array's hash-chain linkage.
lsp-max-scaffold verify chain --file receipts.json
```

Over the LSP wire, every published `Diagnostic` carries its proof in `data`:
`{ "receipt": {...}, "witness": {...} }` — any client can re-verify.

## Open axes (what blocks ADMITTED)

| Axis | State | Blocking precondition |
|------|-------|-----------------------|
| Replay (single-impl) | ADMITTED | self-replay reproduces digests (tested) |
| Replay (cross-impl) | UNKNOWN | a second, independent analyzer impl must reproduce digests |
| Receipt signing | OPEN | receipts are unsigned; add ed25519 over `chain_digest` |
| Chain persistence | OPEN | per-document chains are rebuilt per event, not durably stored |
| Negative control | CANDIDATE | `fixtures/negative_controls/` present; not yet receipt-linked |

`UNKNOWN` and `OPEN` axes do not collapse into `ADMITTED`. The mechanism is
sound and tested; the law status remains bounded until the open axes close.
