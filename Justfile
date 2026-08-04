set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just help

list:
    @just --list

audit:
    @bash scripts/audit-repository.sh

fmt:
    cargo fmt --all

check:
    cargo check --workspace --all-targets

test:
    cargo test --workspace --all-targets

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

ci: audit dx dx-verify

dx:
    cargo fmt --all --check
    cargo check --workspace --all-targets
    cargo test --workspace --all-targets
    cargo clippy --workspace --all-targets --all-features -- -D warnings

dx-verify:
    @bash scripts/doctor.sh
    @bash scripts/doctor-strict.sh

dx-polish:
    cargo fmt --all
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test-pre-publish:
    just audit
    just dx-verify
    just dx-polish
    cargo test --workspace --all-targets -- --include-ignored

release-version-bump VERSION:
    cargo +stable set-version {{VERSION}} --workspace

release-validate:
    just v26-gate-json
    just audit
    just doctor
    just doctor-strict
    just dx-verify
    just dx-polish
    just test-pre-publish
    just release-dry-run

release-dry-run:
    just publish-dry-run

release-publish VERSION:
    @echo "REFUSED: repository automation has no authority to publish v{{VERSION}}"
    @echo "Run the documented human release procedure outside agent and CI execution."
    @exit 64

qol: q failset receipts receipts-check agents-loc agents-closure-scan tree changed clean

v26-gate-json:
    @bash scripts/v26-gate.sh

v26-verify:
    @just release-validate

doctor:
    @bash scripts/doctor.sh

doctor-strict:
    @bash scripts/doctor.sh --strict
    cargo test --workspace --all-targets --jobs 1 -- --test-threads=1
    cargo clippy --workspace --all-targets --all-features --jobs 1 -- -D warnings

lsif:
    @echo "lsif"

lsif-receipt:
    @echo "lsif-receipt"

stale-lsif:
    @echo "stale-lsif"

semantic-graph:
    @echo "semantic-graph"

disclaimer:
    @echo "disclaimer"

rice:
    @echo "rice"

closure-channel:
    @echo "closure-channel"

publish-dry-run:
    cargo publish -p lsp-max --dry-run --allow-dirty

q:
    @bash scripts/q.sh

failset:
    @bash scripts/failset.sh

receipts:
    @ls -l receipts/

receipts-check:
    @bash scripts/receipts-check.sh

agents-loc:
    @wc -l AGENTS.md | awk '{if ($1 <= 200) exit 0; else {print "AGENTS.md > 200 lines"; exit 1}}'

agents-closure-scan:
    @bash scripts/closure-token-scan.sh

tree:
    @tree .

changed:
    @git status -s

clean:
    cargo clean

help:
    @just --list
