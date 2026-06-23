# The Phase Transition of Language

A doctoral thesis (design-science monograph) on the convergence of the Language
Server Protocol, the Model Context Protocol, and the Agent-to-Agent protocol into
a universal **law-state runtime**, framed through Wil van der Aalst's
process-mining paradigm. Prepared for the consideration of
Prof. dr. ir. Wil M. P. van der Aalst.

This directory is the LaTeX source. There is no TeX toolchain in the authoring
environment, so the PDF must be compiled by the reader (instructions below). The
LaTeX is the canonical artifact; this README is a map.

## Central thesis

When LSP is generalised from source code to **all** forms of language and fused
with MCP and A2A, the addressable surface of machine-mediated language undergoes a
*phase transition* — a discontinuous, latent-heat-driven expansion of the order of
the ~1,600–1,700× expansion of water into steam — and the abstraction that makes
the expansion trustworthy is **conformance checking**: every language, once
recorded as an object-centric event log (OCEL 2.0), acquires a conformance
surface. The artifact `lsp-max` (CalVer 26.6.18) is the running instantiation.

## How to build

The source uses `biblatex` with the `biber` backend.

**Overleaf (recommended):** upload the `thesis/` directory, set the main document
to `thesis.tex`, and ensure the bibliography backend is Biber (Overleaf default).

**Local:**

```sh
cd thesis
latexmk -pdf -bibtex- thesis.tex     # latexmk runs biber automatically
# or, manually:
pdflatex thesis && biber thesis && pdflatex thesis && pdflatex thesis
```

Required packages (all in TeX Live / Overleaf): `geometry`, `setspace`,
`fancyhdr`, `amsmath/amssymb/amsthm`, `mathtools`, `tikz`,
`tikz-cd`, `booktabs`, `tabularx`, `longtable`, `listings`, `biblatex`,
`hyperref`, `cleveref`, `epigraph`, `enumitem`, `csquotes`, `microtype`.

## Structure

- `thesis.tex` — master document: preamble, title page, abstract, a note on
  method and language, and the chapter `\input`s. Chapter numbers follow `\input`
  order, not file names.
- `references.bib` — bibliography. Foundational sources (process mining, LSP, MCP,
  A2A, physics, design science) plus ~70 arXiv preprints from 2025-12 to 2026-06.

| # | Chapter | File |
|---|---|---|
| 1 | Introduction | `chapters/01-introduction.tex` |
| 2 | Background & Related Work (+ Recent Advances) | `chapters/02-background.tex`, `chapters/02b-recent-advances.tex` |
| 3 | Mathematical Foundations, from First Principles | `chapters/02a-math-foundations.tex` (+ `02a-algebra`, `02a-order-logic`, `02a-analysis`, `02a-geometry`, `02a-measure`, `02a-synthesis`) |
| 4 | Conceptual Framework: Language as Process | `chapters/03-conceptual-framework.tex` |
| 5 | Research Methodology | `chapters/04-methodology.tex` |
| 6 | The `lsp-max` Artifact | `chapters/05-artifact.tex` |
| 7 | The Fusion Architecture: LSP × MCP × A2A | `chapters/06-fusion.tex` |
| 8 | Conformance Checking for All Language | `chapters/07-all-language.tex` |
| 9 | Vision 2030 | `chapters/08-vision-2030.tex` |
| 10 | Discussion | `chapters/09-discussion.tex` |
| 11 | Conclusion & Future Work | `chapters/10-conclusion.tex` |
| A | The `max/*` Method Catalogue | `chapters/A-method-catalog.tex` |
| B | Laws, Statuses, and Diagnostic Families | `chapters/B-law-table.tex` |
| C | A Protocol Timeline, 2016–2030 | `chapters/C-timeline.tex` |

The five pillars of Chapter 3 derive, from sets and axioms: the **algebra** of
composition (monoids → categories → functors/adjunctions → monoidal categories →
operads, ending in the Decoupling Theorem); the **order-theoretic logic** of
conformance (lattices → Kleene/Belnap many-valued logic → the non-collapse
theorem); the **analysis** of phase transitions (limits → Ehrenfest → a
from-scratch Clausius–Clapeyron derivation → a Landau first-order jump); the
**geometry** of law-state (manifolds → connection/curvature → gradient flow,
formalizing `AGENTS.md`'s manifold metaphors); and **measure & information**
(σ-algebras → entropy → Fisher–Rao, making receipts a literal measure and
non-collapse a data-processing inequality). A closing **synthesis** proves a single
*Conformance Functor* over all seven language domains.

## Epistemic discipline

The thesis adopts the artifact's own constitution: claims carry **bounded status**
(`ADMITTED` / `REFUSED` / `UNKNOWN` / `CANDIDATE` / `PARTIAL` / `OPEN`), the three
conformance verdicts never collapse, forecasts are marked as forecasts, and the
language of triumph is declined.

## Citation-verification note

The ~70 recent-arXiv citations were located by a machine-assisted harvest
restricted to the window 2025-12-01 to 2026-06-21. **15 were independently
verified by hand** against their `arxiv.org/abs/` pages (identifier, title,
authors, v1 date), all confirmed; the remainder rest on the harvest's per-item
fetch. As living preprints they carry bounded status `CANDIDATE`; **re-verify all
before formal submission.** Foundational (pre-window) sources — including
van der Aalst's *No AI Without PI!* (arXiv:2508.00116), the OCEL 2.0 spec
(arXiv:2403.01975), and the primary LSP/MCP/A2A specifications — are cited
directly.

## Status of the work (bounded)

- Structural unification of LSP/MCP/A2A and the Decoupling Theorem — `ADMITTED` (analytical).
- The event-log criterion for "a servable language" — `ADMITTED` (as a criterion).
- The phase-transition model and its mathematics — `ADMITTED` (as a model); 2030 instantiation — `FORECAST`.
- The `lsp-max` law-state runtime realizing the four defining properties — `ADMITTED`, with `OPEN` sub-items (e.g. subagent gate propagation).
- Conformance-for-all-language across six domains — `CANDIDATE` (illustrative; field validation `OPEN`).
