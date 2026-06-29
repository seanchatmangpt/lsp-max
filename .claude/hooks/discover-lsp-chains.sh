#!/bin/sh
# .claude/hooks/discover-lsp-chains.sh
# CC-001: 3-strategy LSP chain discovery
#
# Strategy 1 — Procfs/ps: scan ps aux for known LSP server process names
# Strategy 2 — Socket scan: look for Unix domain sockets matching LSP patterns
# Strategy 3 — Claude Code config: read settings.json / CLAUDE_LSP_SERVERS env var
#
# Output: JSON array to stdout:
#   [{"id":"rust-analyzer","command":"rust-analyzer","args":[],"priority":"primary","extensions":[".rs"]}]
#
# Exit 0 on success (even if empty array); exit 1 only on hard failure.

set -eu

# ---------------------------------------------------------------------------
# Known server registry: "process_name:id:priority:ext1,ext2,..."
# ---------------------------------------------------------------------------
KNOWN_SERVERS="
rust-analyzer:rust-analyzer:primary:.rs
pyright-langserver:pyright:primary:.py,.pyi
typescript-language-server:tsserver:primary:.ts,.tsx,.js,.jsx
clangd:clangd:primary:.c,.cpp,.cc,.h,.hpp
gopls:gopls:primary:.go
zls:zls:primary:.zig
lua-language-server:lua-ls:secondary:.lua
bash-language-server:bash-ls:secondary:.sh,.bash
yaml-language-server:yaml-ls:secondary:.yaml,.yml
json-languageserver:json-ls:secondary:.json
vscode-html-languageserver:html-ls:secondary:.html,.htm
vscode-css-languageserver:css-ls:secondary:.css,.scss,.less
solargraph:solargraph:secondary:.rb
jdtls:jdtls:secondary:.java
metals:metals:secondary:.scala
"

# Temp file accumulates: id<TAB>command<TAB>args<TAB>priority<TAB>extensions
TMPFILE=$(mktemp 2>/dev/null || printf '/tmp/discover-lsp-chains-%s' "$$")
trap 'rm -f "$TMPFILE"' EXIT

# Record a server entry; deduplicate by id.
# $1=id $2=command $3=args(space-sep) $4=priority $5=extensions(comma-sep)
record() {
    _rid="$1"; _rcmd="$2"; _rargs="$3"; _rprio="$4"; _rexts="$5"
    grep -q "^${_rid}	" "$TMPFILE" 2>/dev/null && return 0
    printf '%s\t%s\t%s\t%s\t%s\n' "$_rid" "$_rcmd" "$_rargs" "$_rprio" "$_rexts" >> "$TMPFILE"
}

# ---------------------------------------------------------------------------
# Strategy 1 — Procfs / ps scan
# ---------------------------------------------------------------------------
strategy_ps() {
    _ps_out=$(ps aux 2>/dev/null | tail -n +2 | head -50) || return 0
    printf '%s\n' "$KNOWN_SERVERS" | while IFS=: read -r _proc _id _prio _exts; do
        [ -z "$_proc" ] && continue
        _line=$(printf '%s\n' "$_ps_out" | grep -F "$_proc" | head -1) || true
        [ -z "$_line" ] && continue
        _cmd=$(printf '%s\n' "$_line" | awk '{print $11}')
        [ -z "$_cmd" ] && _cmd="$_proc"
        record "$_id" "$_cmd" "" "$_prio" "$_exts"
    done
}

# ---------------------------------------------------------------------------
# Strategy 2 — Unix domain socket scan
# ---------------------------------------------------------------------------
strategy_sockets() {
    for _dir in /tmp /run "${HOME}/.cache"; do
        [ -d "$_dir" ] || continue
        _sockets=$(find "$_dir" -maxdepth 3 -type s 2>/dev/null | head -10) || continue
        [ -z "$_sockets" ] && continue
        printf '%s\n' "$KNOWN_SERVERS" | while IFS=: read -r _proc _id _prio _exts; do
            [ -z "$_proc" ] && continue
            printf '%s\n' "$_sockets" | grep -iF "$_proc" | head -1 | grep -q . || continue
            _cmd=$(command -v "$_proc" 2>/dev/null) || _cmd="$_proc"
            record "$_id" "$_cmd" "--stdio" "$_prio" "$_exts"
        done
    done
}

# ---------------------------------------------------------------------------
# Strategy 3 — Claude Code config + CLAUDE_LSP_SERVERS env var
# ---------------------------------------------------------------------------

# Parse "id:command [args]" comma-separated entries from CLAUDE_LSP_SERVERS format
parse_env_lsp_servers() {
    _raw="$1"
    # Use tr to split on commas; iterate line by line
    printf '%s' "$_raw" | tr ',' '\n' | while IFS= read -r _entry; do
        [ -z "$_entry" ] && continue
        _eid=$(printf '%s' "$_entry" | cut -d: -f1)
        _erest=$(printf '%s' "$_entry" | cut -d: -f2-)
        _ecmd=$(printf '%s' "$_erest" | awk '{print $1}')
        _eargs=$(printf '%s' "$_erest" | cut -s -d' ' -f2-)
        # Look up metadata from known list
        _eprio="secondary"; _eexts=""
        printf '%s\n' "$KNOWN_SERVERS" | while IFS=: read -r _proc _kid _kprio _kexts; do
            [ -z "$_proc" ] && continue
            case "$_eid" in *"$_proc"*|*"$_kid"*)
                _eprio="$_kprio"; _eexts="$_kexts"; break ;;
            esac
        done
        record "$_eid" "${_ecmd:-$_eid}" "${_eargs:-}" "$_eprio" "$_eexts"
    done
}

strategy_claude_config() {
    # Check CLAUDE_LSP_SERVERS env var first (Strategy 1b per ticket)
    if [ -n "${CLAUDE_LSP_SERVERS:-}" ]; then
        parse_env_lsp_servers "$CLAUDE_LSP_SERVERS"
    fi

    # Candidate config file locations
    for _cfg in \
        "${HOME}/.claude/settings.json" \
        "${HOME}/.config/claude-code/settings.json" \
        "${HOME}/.config/Claude/settings.json" \
        "${CLAUDE_CONFIG_PATH:-/dev/null}"
    do
        [ -f "$_cfg" ] || continue

        if command -v jq >/dev/null 2>&1; then
            # Extract lspServers array entries
            jq -r '
                (.lspServers // []) |
                .[] |
                [ (.id // .name // ""),
                  (.command // ""),
                  ((.args // []) | join(" ")),
                  (.priority // "secondary"),
                  ((.extensions // []) | join(","))
                ] | @tsv
            ' "$_cfg" 2>/dev/null | while IFS='	' read -r _id _cmd _args _prio _exts; do
                [ -z "$_id" ] && continue
                record "$_id" "$_cmd" "$_args" "$_prio" "$_exts"
            done

            # Also check for env var embedded in config
            _env_val=$(jq -r '.env.CLAUDE_LSP_SERVERS // empty' "$_cfg" 2>/dev/null) || true
            [ -n "$_env_val" ] && parse_env_lsp_servers "$_env_val"
        else
            # No jq: grep for known binary names
            printf '%s\n' "$KNOWN_SERVERS" | while IFS=: read -r _proc _id _prio _exts; do
                [ -z "$_proc" ] && continue
                grep -qF "$_proc" "$_cfg" 2>/dev/null || continue
                _cmd=$(command -v "$_proc" 2>/dev/null) || _cmd="$_proc"
                record "$_id" "$_cmd" "" "$_prio" "$_exts"
            done
        fi
    done
}

# ---------------------------------------------------------------------------
# Run all three strategies
# ---------------------------------------------------------------------------
strategy_ps
strategy_sockets
strategy_claude_config

# ---------------------------------------------------------------------------
# Emit JSON array from accumulated results
# ---------------------------------------------------------------------------
printf '['
_first=1
while IFS='	' read -r _id _cmd _args _prio _exts; do
    [ -z "$_id" ] && continue

    # Build extensions JSON array
    _ext_json='['
    _efirst=1
    _save_ifs="$IFS"
    IFS=','
    for _e in $_exts; do
        [ -z "$_e" ] && continue
        if [ "$_efirst" = "1" ]; then
            _ext_json="${_ext_json}\"${_e}\""; _efirst=0
        else
            _ext_json="${_ext_json},\"${_e}\""
        fi
    done
    IFS="$_save_ifs"
    _ext_json="${_ext_json}]"

    # Build args JSON array
    _args_json='['
    _afirst=1
    IFS=' '
    for _a in $_args; do
        [ -z "$_a" ] && continue
        if [ "$_afirst" = "1" ]; then
            _args_json="${_args_json}\"${_a}\""; _afirst=0
        else
            _args_json="${_args_json},\"${_a}\""
        fi
    done
    IFS="$_save_ifs"
    _args_json="${_args_json}]"

    if [ "$_first" = "1" ]; then
        _first=0
    else
        printf ','
    fi
    printf '{"id":"%s","command":"%s","args":%s,"priority":"%s","extensions":%s}' \
        "$_id" "$_cmd" "$_args_json" "$_prio" "$_ext_json"
done < "$TMPFILE"
printf ']\n'
