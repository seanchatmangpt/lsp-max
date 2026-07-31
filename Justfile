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

release-publish VERSION:
    @echo "Publishing v{{VERSION}} to crates.io..."
    @if [ -z "$CARGO_TOKEN" ]; then \
        echo "Error: CARGO_TOKEN environment variable not set"; \
        exit 1; \
    fi
    cargo publish -p lsp-max-protocol --token $CARGO_TOKEN
    @echo "Waiting for lsp-max-protocol to index..."
    @sleep 15
    cargo publish -p lsp-max-macros --token $CARGO_TOKEN
    @echo "Waiting for lsp-max-macros to index..."
    @sleep 15
    cargo publish -p lsp-max-ast --token $CARGO_TOKEN
    @echo "Waiting for lsp-max-ast to index..."
    @sleep 15
    cargo publish -p lsp-max-compositor --token $CARGO_TOKEN
    @echo "Waiting for lsp-max-compositor to index..."
    @sleep 15
    cargo publish -p lsp-max-lsif --token $CARGO_TOKEN
    @echo "Waiting for lsp-max-lsif to index..."
    @sleep 15
    cargo publish -p lsp-max-cli --token $CARGO_TOKEN
    @echo "Waiting for lsp-max-cli to index..."
    @sleep 15
    cargo publish --token $CARGO_TOKEN
    @echo "✓ All crates published for v{{VERSION}}"

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

ggen-sync:
    ${GGEN_BIN:-ggen} sync run

ggen-semantic:
    python3 scripts/verify-ggen-contract-closure.py --check --negative-fixtures

ggen-receipt:
    python3 scripts/verify-ggen-contract-closure.py --check --negative-fixtures --emit-receipt target/ggen/verification-receipt.json
    python3 scripts/verify-ggen-contract-closure.py --replay target/ggen/verification-receipt.json

ggen-rust-contract:
    cargo test --manifest-path tests/ggen-contract/Cargo.toml --test ggen_law_contract

ggen-drift:
    ${GGEN_BIN:-ggen} sync run
    git diff --exit-code -- docs/generated/GGEN_MANUFACTURING_CONTRACT.md docs/generated/STANDING_MODEL.md docs/generated/GALL_ROADMAP.md evidence/generated/verification-manifest.json tests/ggen_law_contract.rs .github/workflows/ggen-contract.yml docs/generated/CMD_PROFILE.md evidence/generated/cmd-contract.json .github/workflows/cmd-contract.yml

ggen-contract: ggen-receipt ggen-rust-contract

ggen-ci: ggen-contract ggen-drift

cmd-observe:
    python3 scripts/verify-cmd-profile-closure.py --suite all --output target/cmd
    @cat target/cmd/repository-observation.json

cmd-fence:
    python3 scripts/verify-cmd-profile-closure.py --suite all --output target/cmd
    @cat target/cmd/authority-map.json

cmd-gates:
    python3 scripts/verify-cmd-profile-closure.py --suite all --output target/cmd
    @cat target/cmd/verifier-report.json

cmd-coverage:
    python3 scripts/verify-cmd-profile-closure.py --suite all --output target/cmd
    @cat target/cmd/candidate-coverage.json

cmd-unit:
    python3 scripts/verify-cmd-profile-closure.py --suite all --output target/cmd

cmd-property: cmd-unit

cmd-integration: cmd-unit

cmd-e2e: cmd-unit

cmd-security: cmd-unit

cmd-chaos: cmd-unit

cmd-stress: cmd-unit

cmd-benchmark: cmd-unit

cmd-receipt:
    python3 scripts/verify-cmd-profile-closure.py --suite all --output target/cmd --emit-receipt target/cmd/cmd-receipt.json

cmd-replay: cmd-receipt
    python3 scripts/verify-cmd-profile-closure.py --replay target/cmd/cmd-receipt.json --output target/cmd/replay

cmd-report: cmd-replay
    @cat target/cmd/verifier-report.json

cmd-crown: ggen-ci cmd-report
    @echo "CROWN UNKNOWN until exact-head CI, detached clean-tree replay, and external evidence close"

tree:
    @tree .

changed:
    @git status -s

clean:
    cargo clean

help:
    @just --list
