set shell := ["bash", "-c"]

# ANSI Colors
export RED := '\033[0;31m'
export GREEN := '\033[0;32m'
export BLUE := '\033[0;34m'
export YELLOW := '\033[1;33m'
export MAGENTA := '\033[0;35m'
export CYAN := '\033[0;36m'
export NC := '\033[0m'

# Default target: list all available commands
default:
    @just --list

# --- Admission ---

# Run perf_refactors benchmarks and write BLAKE3-signed admission receipt
bench-admit:
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${CYAN} Bench-Admit: Performance Refactors Admission Gate ${NC}"
    @echo -e "${MAGENTA}============================================================${NC}"
    @mkdir -p receipts
    @bash scripts/write_bench_receipt.sh
    @echo -e "${GREEN}✓ Receipt written. Run 'just test-perf-admission' to validate.${NC}"

# Validate the perf-refactors receipt exists and is structurally sound
test-perf-admission:
    cargo test --test test_perf_admission -- --nocapture

# Run compositor micro-benchmarks and write BLAKE3-signed admission receipt
bench-compositor:
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${CYAN} Bench-Compositor: Scale Admission Gate ${NC}"
    @echo -e "${MAGENTA}============================================================${NC}"
    @mkdir -p receipts
    @bash scripts/write_compositor_bench_receipt.sh
    @echo -e "${GREEN}✓ Receipt written. Run 'just test-compositor-admission' to validate.${NC}"

# Validate the compositor-scale receipt exists and is structurally sound
test-compositor-admission:
    cargo test --test test_compositor_perf_admission -- --nocapture

# --- Tests ---

# Run all workspace tests
test:
    cargo test --workspace

# Run end-to-end tests
test-e2e:
    cargo test --test e2e

# Run heavy pre-publish validation tests (including AST codegen)
test-pre-publish: dx-verify dx-polish
    cargo test --workspace -- --include-ignored

# --- AutoDX (Developer Experience) ---

# Enforces strict architectural boundaries (no legacy, no shims)
dx-verify:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "============================================================"
    echo " AutoDX: Architectural Boundary Verification"
    echo "============================================================"
    echo "➜ Scanning for forbidden legacy residue..."
    FORBIDDEN="legacy|Legacy|LEGACY|deprecated|deprecation|facade|shim|backward compatibility|compatibility layer"
    RESIDUE=$(rg -n "$FORBIDDEN" ../wasm4pm-compat ../wasm4pm \
        --glob '!target/**' --glob '!target_lsp/**' --glob '!.git/**' \
        --glob '!Cargo.lock' --glob '!package-lock.json' --glob '!pnpm-lock.yaml' \
        --glob '!**/node_modules/**' --glob '!paper/**' --glob '!**/docs/**' \
        --glob '!**/receipts/**' --glob '!**/*.md' --glob '!**/*.tex' --glob '!**/*.log' \
        || true)
    if [ -n "$RESIDUE" ]; then
        echo "✗ Forbidden residue found! Architecture compromised."
        echo "$RESIDUE" | head -n 15
        exit 1
    fi
    echo "✓ No legacy residue found."
    echo "➜ Scanning for forbidden intermediary type authorities (wasm4pm_types, ocel_core)..."
    AUTHORITIES=$(rg -n "wasm4pm_types|ocel_core" ../wasm4pm-compat ../wasm4pm \
        --glob '!target/**' --glob '!target_lsp/**' --glob '!.git/**' \
        --glob '!Cargo.lock' --glob '!package-lock.json' --glob '!pnpm-lock.yaml' \
        --glob '!**/node_modules/**' --glob '!paper/**' --glob '!**/docs/**' \
        --glob '!**/receipts/**' --glob '!**/*.md' --glob '!**/*.tex' --glob '!**/*.log' \
        || true)
    if [ -n "$AUTHORITIES" ]; then
        echo "✗ Forbidden type authorities found! wasm4pm-compat is the sole baseline."
        echo "$AUTHORITIES" | head -n 15
        exit 1
    fi
    echo "✓ Type authorities are mathematically sound."

# Runs formatters, linters, and strict clippy checks
dx-polish:
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${CYAN} 🚀 AutoDX: Codebase Polish & Linting ${NC}"
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${BLUE}➜ Running cargo fmt across workspace...${NC}"
    cargo fmt --all
    @echo -e "${BLUE}➜ Running strict clippy checks...${NC}"
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    @echo -e "${GREEN}✓ Codebase is polished and lint-free.${NC}"

# Runs polish, verify, clean, and intel in sequence
dx-all: dx-polish dx-verify qol-clean etc-intel

# --- AutoQoL (Quality of Life) ---

# Garbage collects cargo cache and massive target directories
qol-clean:
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${CYAN} 🚀 AutoQoL: Ecosystem Garbage Collection ${NC}"
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${BLUE}➜ Pruning Cargo cache...${NC}"
    @cargo cache -a || (cargo install cargo-cache && cargo cache -a)
    @echo -e "${BLUE}➜ Cleaning deep targets (retaining incremental)...${NC}"
    @TARGET_SIZE=$$(du -sm target 2>/dev/null | awk '{print $$1}' || echo "0"); \
    if [ "$$TARGET_SIZE" -gt 10000 ]; then \
        echo -e "${BLUE}➜ Target dir > 10GB. Executing deep clean...${NC}"; \
        cargo clean; \
    else \
        echo -e "${GREEN}✓ Target dir is within acceptable limits ($${TARGET_SIZE}MB).${NC}"; \
    fi

# Fetches and prunes git state across the entire wasm4pm ecosystem
qol-sync:
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${CYAN} 🚀 AutoQoL: Cross-Ecosystem Synchronization ${NC}"
    @echo -e "${MAGENTA}============================================================${NC}"
    @for DIR in . ../wasm4pm ../wasm4pm-compat; do \
        if [ -d "$$DIR" ]; then \
            echo -e "${YELLOW}Syncing [$$DIR]...${NC}"; \
            (cd "$$DIR" && git fetch --all --prune && git status -s); \
        fi; \
    done
    @echo -e "${GREEN}✓ Ecosystem sync complete.${NC}"

# --- Spec Graph ---

# Regenerate LSP 3.18 spec-graph artifacts from the canonical metaModel.json
spec-graph:
    cargo run -p lsp-max-specgen -- \
        --emit-spec-graph examples/anti-llm-lsp/generated \
        --input vendors/vscode-languageserver-node/protocol/metaModel.json

# --- DX Scripts (Developer Experience Tooling) ---

# Check codebase law compliance (tower_lsp refs, victory language, etc)
check-laws:
    @bash scripts/check-law-compliance.sh

# Analyze hot-path benchmarks for regressions (if benches/ exists)
bench-hot:
    @bash scripts/bench-hot-paths.sh

# Run format and clippy checks (-D warnings)
check-fmt:
    @bash scripts/format-and-check.sh

# Update doc coverage metrics and append to DOC_COVERAGE_LOG.md
doc-coverage:
    @bash scripts/update-doc-coverage.sh

# Run all health checks in sequence (law, format, bench, doc-coverage)
health-check:
    @bash scripts/health-check.sh

# --- AutoEtc (Operational & Context) ---

# Generates context manifests for Generative AI / AGI consumption
etc-intel:
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${CYAN} 🚀 AutoEtc: Generative AI Intel Manifest ${NC}"
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${BLUE}➜ Generating ecosystem structural map for AGI...${NC}"
    @mkdir -p .autodx
    @echo "# Ecosystem AGI Manifest" > .autodx/ecosystem-intel.md
    @echo "Generated automatically by Justfile." >> .autodx/ecosystem-intel.md
    @echo "" >> .autodx/ecosystem-intel.md
    @echo "## Architectural Mandate" >> .autodx/ecosystem-intel.md
    @echo "- **wasm4pm-compat**: The sole, axiomatic baseline type authority." >> .autodx/ecosystem-intel.md
    @echo "- **wasm4pm**: The execution engine." >> .autodx/ecosystem-intel.md
    @echo "- **lsp-max**: The semantic intelligence layer." >> .autodx/ecosystem-intel.md
    @echo "" >> .autodx/ecosystem-intel.md
    @echo "## Forbidden Concepts" >> .autodx/ecosystem-intel.md
    @echo "- No deprecation paths." >> .autodx/ecosystem-intel.md
    @echo "- No legacy terminology." >> .autodx/ecosystem-intel.md
    @echo "- No intermediary type crates (e.g., wasm4pm-types, ocel-core)." >> .autodx/ecosystem-intel.md
    @echo "" >> .autodx/ecosystem-intel.md
    @echo "## Crate Topography" >> .autodx/ecosystem-intel.md
    @cargo tree --depth 1 >> .autodx/ecosystem-intel.md || true
    @echo -e "${GREEN}✓ Intel manifest generated at .autodx/ecosystem-intel.md${NC}"

# --- Release Automation (see docs/RELEASE.md) ---

# Validate all pre-release conditions (gate, verify, polish, tests, sync)
release-validate:
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  Release Validation Checklist${NC}"
    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"

    echo -e "${BLUE}► Gate check...${NC}"
    cargo run -p lsp-max-cli -- gate check || { echo -e "${RED}✗ Gate is SET${NC}"; exit 1; }

    echo -e "${BLUE}► Boundaries (dx-verify)...${NC}"
    just dx-verify

    echo -e "${BLUE}► Code polish (dx-polish)...${NC}"
    just dx-polish

    echo -e "${BLUE}► Full test suite...${NC}"
    just test-pre-publish

    echo -e "${BLUE}► Sibling repos synced...${NC}"
    just qol-sync

    echo -e "${GREEN}✓ All pre-release checks passed${NC}"

# Dry-run publish for all crates (no token required)
release-dry-run:
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  Dry-Run Publish (No Credentials Required)${NC}"
    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"

    for crate in lsp-max-protocol lsp-max-runtime lsp-max-agent lsp-max-macros lsp-max; do
        echo -e "${BLUE}► $crate${NC}"
        cargo publish -p "$crate" --dry-run
    done

    echo -e "${GREEN}✓ All crates ready to publish${NC}"

# Publish to crates.io in dependency order (requires CARGO_TOKEN)
release-publish VERSION:
    #!/usr/bin/env bash
    set -euo pipefail

    if [ -z "${CARGO_TOKEN:-}" ]; then
        echo -e "${RED}✗ CARGO_TOKEN not set${NC}"
        exit 1
    fi

    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  Publishing lsp-max {{ VERSION }}${NC}"
    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"

    mkdir -p receipts
    CHECKSUM_FILE="receipts/publish-checksums-{{ VERSION }}.txt"
    > "$CHECKSUM_FILE"  # Clear file

    for crate in lsp-max-protocol lsp-max-runtime lsp-max-agent lsp-max-macros lsp-max; do
        echo -e "${BLUE}► Publishing $crate @ {{ VERSION }}...${NC}"
        cargo publish -p "$crate" --token "$CARGO_TOKEN"

        # Wait for crates.io to index (poll up to 2 minutes)
        attempt=0
        max_attempts=30
        while [ $attempt -lt $max_attempts ]; do
            if curl -s "https://crates.io/api/v1/crates/$crate/{{ VERSION }}" \
                | grep -q "{{ VERSION }}"; then
                echo -e "${GREEN}  ✓ Indexed on crates.io${NC}"

                # Record checksum
                CHECKSUM=$(curl -s "https://crates.io/api/v1/crates/$crate/{{ VERSION }}" \
                    | jq -r '.version.checksum')
                echo "$crate | {{ VERSION }} | $CHECKSUM" >> "$CHECKSUM_FILE"
                break
            fi
            attempt=$((attempt + 1))
            sleep 4
        done

        if [ $attempt -eq $max_attempts ]; then
            echo -e "${RED}✗ Timeout waiting for $crate to index${NC}"
            exit 1
        fi
    done

    echo -e "${GREEN}✓ All crates published${NC}"
    echo -e "${BLUE}Checksums saved to: $CHECKSUM_FILE${NC}"

# Bump workspace version to CalVer format (e.g., 26.7.15)
release-version-bump NEWVERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  Bumping version to {{ NEWVERSION }}${NC}"
    echo -e "${MAGENTA}════════════════════════════════════════════════════════${NC}"

    cargo set-version {{ NEWVERSION }} --workspace

    # Verify all path deps updated
    echo -e "${BLUE}► Verifying path dependencies...${NC}"
    for crate in lsp-max-protocol lsp-max-runtime lsp-max-agent lsp-max-macros; do
        count=$(grep -r "version = \"{{ NEWVERSION }}\"" . --include="Cargo.toml" | grep "$crate" | wc -l)
        if [ "$count" -eq 0 ]; then
            echo -e "${RED}✗ $crate not updated${NC}"
            exit 1
        fi
    done

    # Run diagnostic canary
    echo -e "${BLUE}► Running diagnostic canary...${NC}"
    cargo run -p anti-llm-cheat-lsp -- check || exit 1

    git add Cargo.toml */Cargo.toml */*/Cargo.toml
    git commit -m "chore: bump version to {{ NEWVERSION }} for release

CalVer (YY.M.D) convention enforced by ANTI-LLM-VERSION-* diagnostics.

https://claude.ai/code/session_01ESRv2v2dcXUvJj7VpkohkY"

    echo -e "${GREEN}✓ Version bumped to {{ NEWVERSION }}${NC}"

# Extract release notes from DOC_COVERAGE_LOG by date
release-notes-extract DATE VERSION:
    #!/usr/bin/env bash
    bash scripts/extract-release-notes.sh "{{ DATE }}" "{{ VERSION }}"
