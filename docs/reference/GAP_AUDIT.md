# lsp-max Gap Audit

Status vocabulary: `UNKNOWN`, `PARTIAL_ALIVE`, `ALIVE`, `BLOCKED`, `BUILD_BROKEN`, `UNSUPPORTED`, or a typed `REFUSED` state.

This audit is bounded to repository topology, Cargo metadata, compiler/runtime claims, CI authority, build and test gates, release paths, documentation, dependency maintenance, and source surfaces exercised by the exact-head quality ladder. It is not a claim that every behavior in every environment has been exhaustively proven.

## 80/20 repairs implemented

| Priority | Gap | Consequence | Implemented control |
| --- | --- | --- | --- |
| P0 | Machine-local and sibling Cargo paths | Clean checkout could not resolve | Published dependencies and repository-local paths only; repository audit rejects path escape |
| P0 | Credentialed automated publication | Irreversible release actuation without explicit authority | Release workflow is validation-only; real publication is explicitly refused |
| P0 | Fail-open CI constructs | Red checks could be reported green | Authoritative workflows use fail-closed commands and exact exit enforcement |
| P0 | CI built synthetic or ambiguous subjects | Receipts did not bind the published PR head | Pull-request workflows check out `github.event.pull_request.head.sha` and record `HEAD` |
| P1 | No authoritative workspace quality ladder | Build-only success hid lint and test failures | Audit, pinned toolchain, formatting, strict Clippy, workspace tests, docs/doctests, package dry-run, and Docker gates |
| P1 | False compiler support claim | Declared Rust floor did not match resolved dependencies or nightly use | Rust `1.87.0` is the dependency-resolution floor; pinned `nightly-2026-04-15` is executable authority; stable is unsupported |
| P1 | Nonfunctional runtime-neutral feature | Published feature surface did not compile | Removed the selectable feature and async-codec dependency; Tokio is the only admitted runtime |
| P1 | Missing receipt verification tests | Receipt standing lacked direct tamper/replay controls | Added signature, linkage, sequence, replay, tamper, and serde round-trip tests |
| P1 | Stale contributor and user setup | Documentation required obsolete sibling repositories and nonexistent commands | README and CONTRIBUTING now match executable topology and replay commands |
| P1 | Unrelated source deletion in inherited branch | Repair scope silently removed ordinary POWL source | Restored the nested POWL source tree from the exact base SHA |
| P2 | No vulnerability reporting policy | Security reports lacked a private, typed intake boundary | Added `SECURITY.md` with critical receipt, path, protocol, CI, and actuation boundaries |
| P2 | Dependency update blind spot | Cargo and Actions dependencies could silently age | Dependabot covers both ecosystems with bounded grouping |

## Current admitted boundaries

- Pinned `nightly-2026-04-15` compiler identity.
- Tokio runtime feature surface.
- Repository-local path dependencies and registry dependencies.
- Exact-head repository audit, formatting, strict Clippy, workspace targets/tests, documentation/doctests, root package dry-run, law scan, conformance scan, TPOT2 surfaces, ggen artifacts, and clean Docker workspace build when their exact-head runs exit successfully.
- Automated release validation only. Real publication requires separate human authority.

## Residual gaps

### P0/P1 follow-up

1. **Runtime abstraction — `UNSUPPORTED`.** Client, routing, transport, watchdog, composition, and process-mining layers remain Tokio-coupled. A lawful implementation requires explicit runtime traits, bounded spawning/time/I/O authority, feature-isolated compilation, and execution tests for every supported runtime.
2. **Dependency lock and supply-chain policy — `PARTIAL_ALIVE`.** The library workspace resolves against the registry but does not yet provide a committed application lock policy, `cargo-deny` policy, vulnerability audit receipt, provenance/SBOM generation, or license/advisory gate.
3. **Ignored and environment-dependent tests — `UNKNOWN`.** Ignored tests must be inventoried with reasons and executed in dedicated environments rather than counted as ordinary success.
4. **Full end-to-end editor/client interoperability — `PARTIAL_ALIVE`.** Workspace tests and examples are broader than unit coverage, but a versioned interoperability matrix against real LSP clients and transport failure modes remains absent.
5. **Performance standing — `UNKNOWN`.** Bench targets compile, but latency, allocation, backpressure, incremental-update, and large-workspace thresholds are not enforced as regression gates.

### P2 follow-up

6. **Unsafe and panic policy enforcement — `PARTIAL_ALIVE`.** Contributor policy exists, but repository-wide machine enforcement for production `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, and `unimplemented!` has not been admitted with a reviewed exception mechanism.
7. **Generated projection drift — `PARTIAL_ALIVE`.** Law and artifact presence are checked, but every ontology/query/template/generated-source correspondence is not yet rebuilt and diff-verified in one deterministic gate.
8. **Release reproducibility — `PARTIAL_ALIVE`.** Package dry-run validates the root crate; reproducible archives, checksums, signed attestations, dependency-order dry-runs for every publishable crate, and consumer installation tests remain outstanding.
9. **Observability and operational failure drills — `UNKNOWN`.** Structured diagnostics exist, but chaos tests for cancellation storms, channel closure, malformed frames, disk pressure, semantic-store corruption, and receipt replay at scale are incomplete.
10. **API compatibility discipline — `UNKNOWN`.** No admitted semver/API-diff gate currently prevents accidental public API breakage across CalVer releases.

## Falsifiers

An `ALIVE` claim for a boundary is invalidated by any of the following:

- the exact checked-out SHA differs from the reported subject;
- a command was inspected but not executed;
- a workflow masks, pipes away, or ignores a producer exit code;
- a dependency resolves outside the repository without being a registry dependency;
- a selectable feature does not compile in its documented configuration;
- a receipt verifies without binding signature, consequence, sequence, predecessor, and replayed observation;
- a generated artifact is edited without validating its canonical source;
- publication occurs from CI or agent execution without explicit human authority;
- a passing subset is represented as full workspace or end-to-end standing.

## Replay ladder

```bash
bash scripts/audit-repository.sh
cargo +nightly-2026-04-15 fmt --all -- --check
cargo +nightly-2026-04-15 check -p lsp-max --lib
cargo +nightly-2026-04-15 check -p lsp-max --lib --all-features
cargo +nightly-2026-04-15 clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly-2026-04-15 test --workspace --all-targets
cargo +nightly-2026-04-15 test --workspace --doc
cargo +nightly-2026-04-15 publish -p lsp-max --dry-run --allow-dirty
docker build --progress=plain --file Dockerfile.ci --tag lsp-max-rust:local .
```
