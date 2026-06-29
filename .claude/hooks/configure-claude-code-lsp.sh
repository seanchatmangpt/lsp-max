#!/usr/bin/env bash
# configure-claude-code-lsp.sh
# CC-006: Write .claude/settings.json [lsp] section to route Claude Code LSP
# connections through the lsp-max compositor.
#
# Reads: ${CLAUDE_PROJECT_DIR}/.claude/compositor-endpoint.json
# Writes: ${CLAUDE_PROJECT_DIR}/.claude/settings.json (lsp section only)
# Emits:  JSON summary to stdout on success
# Exit 0 on success or graceful skip; exit 1 on error

set -euo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git -C "$(dirname "$0")/../.." rev-parse --show-toplevel 2>/dev/null || pwd)}"
ENDPOINT="${PROJECT_DIR}/.claude/compositor-endpoint.json"
SETTINGS="${PROJECT_DIR}/.claude/settings.json"
TOML="${PROJECT_DIR}/lsp-max.toml"

# Wait up to 5s for the compositor endpoint file to appear.
for i in {1..10}; do
    [[ -f "$ENDPOINT" ]] && break
    sleep 0.5
done

if [[ ! -f "$ENDPOINT" ]]; then
    printf '{"status":"BLOCKED","reason":"compositor endpoint absent after 5s","config_path":"%s"}\n' "$SETTINGS" >&2
    # Graceful degradation — leave Claude Code's config unchanged.
    exit 0
fi

# Read endpoint address from descriptor.
if ! command -v jq &>/dev/null; then
    printf '{"status":"BLOCKED","reason":"jq not found","config_path":"%s"}\n' "$SETTINGS" >&2
    exit 0
fi

ENDPOINT_ADDR=$(jq -r '.endpoint // empty' "$ENDPOINT" 2>/dev/null)
if [[ -z "$ENDPOINT_ADDR" ]]; then
    printf '{"status":"BLOCKED","reason":"endpoint field missing in descriptor","config_path":"%s"}\n' "$SETTINGS" >&2
    exit 0
fi

# Check manage_claude_config flag in lsp-max.toml (default: false).
MANAGE="false"
if [[ -f "$TOML" ]]; then
    MANAGE=$(grep -E 'manage_claude_config\s*=' "$TOML" 2>/dev/null \
        | grep -oE '(true|false)' | head -1 || echo "false")
fi

if [[ "$MANAGE" != "true" ]]; then
    printf '{"status":"SKIPPED","reason":"manage_claude_config=false","endpoint":"%s","config_path":"%s"}\n' \
        "$ENDPOINT_ADDR" "$SETTINGS"
    exit 0
fi

# Build the compositor command entry.
# The compositor listens on a TCP socket; Claude Code drives it via the address.
COMPOSITOR_CMD=$(jq -r '.command // empty' "$ENDPOINT" 2>/dev/null)
if [[ -z "$COMPOSITOR_CMD" ]]; then
    # Fall back: treat endpoint as a socket address; wrap in a socat command.
    COMPOSITOR_CMD="lsp-max-compositor"
fi

# Read current settings.json, or start from an empty object.
if [[ -f "$SETTINGS" ]]; then
    CURRENT=$(cat "$SETTINGS")
else
    CURRENT='{}'
fi

# Collect primary_extensions from the merged registry (lsp-max.toml + lsp-max-auto.toml).
# We look for [[server]] entries with a primary_extension field.
AUTO_TOML="${PROJECT_DIR}/.claude/lsp-max-auto.toml"
EXTENSIONS=()

for toml_file in "$TOML" "$AUTO_TOML"; do
    [[ -f "$toml_file" ]] || continue
    while IFS= read -r ext; do
        EXTENSIONS+=("$ext")
    done < <(grep -E 'primary_extension\s*=' "$toml_file" 2>/dev/null \
        | grep -oE '"[^"]+"' | tr -d '"' || true)
done

# Deduplicate extensions.
mapfile -t EXTENSIONS < <(printf '%s\n' "${EXTENSIONS[@]}" | sort -u)

if [[ ${#EXTENSIONS[@]} -eq 0 ]]; then
    # No extensions found — write a generic rust entry as a baseline.
    EXTENSIONS=("rs")
fi

# Build lsp entries JSON object.
LSP_ENTRIES="{}"
for ext in "${EXTENSIONS[@]}"; do
    LSP_ENTRIES=$(printf '%s' "$LSP_ENTRIES" | jq \
        --arg ext "$ext" \
        --arg cmd "$COMPOSITOR_CMD" \
        '.[$ext] = {"command": $cmd, "args": [], "enabled": true}')
done

# Merge into settings.json: only write keys that are absent (unless manage_claude_config=true,
# which we already gated above). Since we are here, manage=true, so overwrite lsp-max entries.
UPDATED=$(printf '%s' "$CURRENT" | jq \
    --argjson lsp "$LSP_ENTRIES" \
    '.lsp = ((.lsp // {}) * $lsp)')

# Write atomically via a temp file.
TMP_SETTINGS="${SETTINGS}.cc006.tmp"
printf '%s\n' "$UPDATED" > "$TMP_SETTINGS"
mv "$TMP_SETTINGS" "$SETTINGS"

# Emit JSON summary to stdout.
printf '{"status":"ADMITTED","endpoint":"%s","config_path":"%s","extensions":%s}\n' \
    "$ENDPOINT_ADDR" \
    "$SETTINGS" \
    "$(printf '%s\n' "${EXTENSIONS[@]}" | jq -R . | jq -s .)"
