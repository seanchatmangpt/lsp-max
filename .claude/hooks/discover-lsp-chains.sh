#!/usr/bin/env bash
# .claude/hooks/discover-lsp-chains.sh
# SessionStart:startup — scan workspace for installed LSP servers and emit
# .claude/lsp-max-auto.toml for CompositorConfig::load_with_auto() to merge.
#
# Only runs on "startup" matcher (fresh sessions), not on resume/clear/compact,
# so it does not overwrite a hot auto config while the compositor is running.

set -euo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
AUTO_TOML="${PROJECT_DIR}/.claude/lsp-max-auto.toml"

# Truncate; we rewrite from scratch each startup.
: > "$AUTO_TOML"

discovered=()

# rust-analyzer — present if Cargo.toml exists and binary is in PATH or in target/
if [[ -f "${PROJECT_DIR}/Cargo.toml" ]]; then
    if command -v rust-analyzer &>/dev/null || [[ -f "${PROJECT_DIR}/target/debug/rust-analyzer" ]]; then
        CMD=$(command -v rust-analyzer 2>/dev/null || echo "${PROJECT_DIR}/target/debug/rust-analyzer")
        cat >> "$AUTO_TOML" <<TOML
[[server]]
id = "rust-analyzer"
command = "${CMD}"
priority = "full"
primary_extensions = [".rs"]
secondary_extensions = []

TOML
        discovered+=("rust-analyzer")
    fi
fi

# typescript-language-server
if [[ -f "${PROJECT_DIR}/package.json" ]] && command -v typescript-language-server &>/dev/null; then
    cat >> "$AUTO_TOML" <<'TOML'
[[server]]
id = "tsserver"
command = "typescript-language-server"
args = ["--stdio"]
priority = "full"
primary_extensions = [".ts", ".tsx"]
secondary_extensions = [".js", ".jsx"]

TOML
    discovered+=("tsserver")
fi

# pyright (Python)
if { [[ -f "${PROJECT_DIR}/pyproject.toml" ]] || [[ -f "${PROJECT_DIR}/setup.py" ]]; } \
    && command -v pyright-langserver &>/dev/null; then
    cat >> "$AUTO_TOML" <<'TOML'
[[server]]
id = "pyright"
command = "pyright-langserver"
args = ["--stdio"]
priority = "full"
primary_extensions = [".py"]
secondary_extensions = [".pyi"]

TOML
    discovered+=("pyright")
fi

# clangd (C/C++)
if { find "${PROJECT_DIR}" -maxdepth 2 -name "*.c" -o -name "*.cpp" 2>/dev/null | grep -q .; } \
    && command -v clangd &>/dev/null; then
    cat >> "$AUTO_TOML" <<'TOML'
[[server]]
id = "clangd"
command = "clangd"
priority = "full"
primary_extensions = [".c", ".cpp", ".cc", ".h", ".hpp"]
secondary_extensions = []

TOML
    discovered+=("clangd")
fi

# lsp-max-mcp — always register if built; it surfaces the routing API over MCP
if command -v lsp-max-mcp &>/dev/null || [[ -f "${PROJECT_DIR}/target/debug/lsp-max-mcp" ]]; then
    MCP_CMD=$(command -v lsp-max-mcp 2>/dev/null || echo "${PROJECT_DIR}/target/debug/lsp-max-mcp")
    cat >> "$AUTO_TOML" <<TOML
[[server]]
id = "lsp-max-mcp"
command = "${MCP_CMD}"
priority = "full"
primary_extensions = []
secondary_extensions = []

TOML
    discovered+=("lsp-max-mcp")
fi

if [[ ${#discovered[@]} -eq 0 ]]; then
    echo "lsp-max auto-discovery: no external servers found; lsp-max.toml handles project chains"
else
    printf "lsp-max auto-discovery → %s\n" "${discovered[*]}"
    echo "Written: ${AUTO_TOML}"
fi
