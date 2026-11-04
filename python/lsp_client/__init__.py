"""Async stdlib-only LSP client for the branchless-lsp server (lane BL9).

Public surface: `analyze` (see branchless_client). No third-party deps.
"""

from .branchless_client import (
    LspError,
    LspProtocolError,
    LspServerCrashed,
    LspTimeout,
    analyze,
    analyze_text,
)

__all__ = [
    "analyze",
    "analyze_text",
    "LspError",
    "LspProtocolError",
    "LspServerCrashed",
    "LspTimeout",
    "CLIENT_VERSION",
]

CLIENT_VERSION = "0.1.0"
