set shell := ["bash", "-c"]

default:
    @just help

list:
    @just --list

fmt:
    cargo fmt --all

check:
    cargo check --all

test:
    cargo test --all

clippy:
    cargo clippy --all-targets -- -D warnings

ci: dx qol doctor

dx:
    cargo fmt --all --check
    cargo check --all
    cargo test --all
    cargo clippy --all-targets -- -D warnings

dx-verify:
    @bash scripts/doctor.sh
    @bash scripts/doctor-strict.sh

dx-polish:
    cargo fmt --all
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test-pre-publish:
    just dx-verify
    just dx-polish
    cargo test --workspace -- --include-ignored

release-version-bump VERSION:
    cargo +stable set-version {{VERSION}} --workspace

release-validate:
    just v26-gate-json
    just doctor
    just doctor-strict
    just dx-verify
    just dx-polish
    just test-pre-publish

release-dry-run:
    just publish-dry-run

qol: q failset receipts receipts-check agents-loc agents-closure-scan tree changed clean

v26-gate-json:
    @bash scripts/v26-gate.sh

v26-verify:
    @echo "Verifying v26.6.28 components..."
    just v26-gate-json
    just doctor
    just doctor-strict
    just dx
    cargo test --all
    cargo clippy --all-targets -- -D warnings
    cargo publish --dry-run

doctor:
    @bash scripts/doctor.sh

doctor-strict:
    @bash scripts/doctor.sh --strict
    cargo test --all --jobs 1 -- --test-threads=1
    cargo clippy --all-targets --jobs 1 -- -D warnings

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
    cargo publish --dry-run

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
