# Archive

This directory contains historical documentation, superseded design documents, and exploration notes. Content here is not actively maintained and may reference code or decisions that no longer exist.

## What's here

- **`adr/`:** Old ADR (Architecture Decision Record) files, now replaced by `/docs/rfcs/`. Kept for historical context.
- **`law/`:** Theoretical foundations of the law-state runtime, now unified in `docs/book/01-architecture.md`.
- **`reports/`:** Exploration and research documents (BASELINE_TYPE_AUTHORITY_THESIS, BLUE_OCEAN_INNOVATION_THESIS, WASM4PM_COMPAT_THESIS, etc.). These represent one-off investigations, not ongoing design decisions.
- **`dod/`:** Definition of Done (DoD) tickets for completed v26.6.28 epic (Salsa+LSIF integration). Closed out; kept for audit trail.
- **`max-001-rounds/`, `v26.6.5/`:** Older version-specific documentation directories; historical only.

## Migrated Content

The following docs were consolidated into new sources:

- **ARCHITECTURE.md, CHAIN-THEORY.md, explanation.md, docs/law/** → `docs/book/01-architecture.md` (unified narrative)
- **docs/adr/** → `docs/rfcs/` (single-numbered RFC sequence)
- **docs/FEATURES.md** → Consolidated into RFC 0001 and architecture narrative
- **docs/00_READ_ME_FIRST.md** → Replaced by top-level `README.md`

## Using Archive

If you're looking for historical context or exploring why a decision was made, check here. But for current guidance, always refer to:

- **Architecture:** `docs/book/01-architecture.md`
- **Design decisions:** `docs/rfcs/README.md`
- **Contributing:** `/CONTRIBUTING.md`
- **Changelog:** `/CHANGELOG.md`
