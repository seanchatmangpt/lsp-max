# lsp-max-scaffold example

A minimal law-state LSP scaffold generator driven by `ggen` (verified against
installed `ggen 26.10.11`, last-verified 2026-11-03; see
`~/.claude/skills/run-ggen/SKILL.md` for the full invocation contract).

## Layout

    gen.toml          # the ggen manifest (declarative-rules schema)
    ontology/scaffold.ttl
    queries/handlers.sparql
    queries/conformance_init.sparql
    templates/handler_stub.rs.tera
    templates/conformance_init.rs.tera
    src/generated_handlers.rs      # generated — DO NOT HAND-PATCH
    src/generated_conformance.rs   # generated — DO NOT HAND-PATCH

## Invoking ggen

- **No `--manifest` flag exists.** `ggen sync run` reads `ggen.toml` from its
  current working directory only. This example's manifest is `gen.toml`, so
  stage a temporary copy and remove it in the same breath:

      cd examples/lsp-max-scaffold
      cp gen.toml ggen.toml && ggen sync run && rm ggen.toml

  Never leave the staged `ggen.toml` in place.
- **Every `SELECT` needs `ORDER BY`** — `strict_mode` rejects queries without
  one at config-validation time (`error[E0013]`), before the pipeline runs.
- **Template discovery note:** this example uses the declarative-rules schema
  (rules you author). Pack-composition manifests with `[templates] dir`
  discover `.tmpl` files only — a `.tera` file there is silently skipped.
  This example has no `.tmpl` files and no `[templates]` section, so that rule
  does not apply here.
- `output_dir` and `output_file` resolve relative to the manifest's directory;
  keep the whole tree under one directory you `cd` into.

## Regeneration and the source-of-truth law

`src/generated_*.rs` are projections of `ontology/scaffold.ttl` through the
queries and templates. If a generated file disagrees with the ontology, fix
the source and regenerate — never hand-patch the generated file (GGM-SRC-002).
`mode = "Overwrite"` in `gen.toml` enforces this: each sync overwrites the
projection from the current graph.

Deterministic regeneration is a property of the pipeline, not an accident:
running `ggen sync run` twice produces byte-identical output (verified
2026-11-04; content-hash skipping makes the second run a no-op `skipped`).

## Law notes

- Bounded status vocabulary only (ADMITTED/CANDIDATE/REFUSED/BLOCKED/
  UNKNOWN/OPEN) — never collapse Unknown into Admitted or Refused.
- Generated-file banners name the ontology source and the regeneration path;
  no victory language.
