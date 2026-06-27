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

qol: q failset receipts receipts-check agents-loc agents-closure-scan tree changed clean

doctor:
    @bash scripts/doctor.sh

doctor-strict:
    @bash scripts/doctor.sh --strict
    cargo test --all
    cargo clippy --all-targets -- -D warnings

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
