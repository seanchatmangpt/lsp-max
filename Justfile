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
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${CYAN} 🚀 AutoDX: Architectural Boundary Verification ${NC}"
    @echo -e "${MAGENTA}============================================================${NC}"
    @echo -e "${BLUE}➜ Scanning for forbidden legacy residue...${NC}"
    @FORBIDDEN="legacy|Legacy|LEGACY|deprecated|deprecation|facade|shim|backward compatibility|compatibility layer"; \
    RESIDUE=$$(rg -n "$$FORBIDDEN" ../wasm4pm-compat ../wasm4pm --glob '!target/**' --glob '!target_lsp/**' --glob '!.git/**' --glob '!Cargo.lock' --glob '!package-lock.json' --glob '!pnpm-lock.yaml' || true); \
    if [ -n "$$RESIDUE" ]; then \
        echo -e "${RED}✗ Forbidden residue found! Architecture compromised.${NC}"; \
        echo "$$RESIDUE" | head -n 15; \
        exit 1; \
    fi
    @echo -e "${GREEN}✓ No legacy residue found. Ecosystem is pristine.${NC}"
    @echo -e "${BLUE}➜ Scanning for forbidden intermediary type authorities (wasm4pm_types, ocel_core)...${NC}"
    @AUTHORITIES=$$(rg -n "wasm4pm_types|ocel_core" ../wasm4pm-compat ../wasm4pm --glob '!target/**' --glob '!target_lsp/**' --glob '!.git/**' --glob '!Cargo.lock' --glob '!package-lock.json' --glob '!pnpm-lock.yaml' || true); \
    if [ -n "$$AUTHORITIES" ]; then \
        echo -e "${RED}✗ Forbidden type authorities found! wasm4pm-compat is the sole baseline.${NC}"; \
        echo "$$AUTHORITIES" | head -n 15; \
        exit 1; \
    fi
    @echo -e "${GREEN}✓ Type authorities are mathematically sound.${NC}"

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
