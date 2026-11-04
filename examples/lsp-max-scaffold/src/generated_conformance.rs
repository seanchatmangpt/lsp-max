// GENERATED FILE — DO NOT HAND-PATCH.
// Projection of examples/lsp-max-scaffold/ontology/scaffold.ttl via gen.toml rule
// `scaffold-conformance`. Regenerate: see README.md (regeneration section).
// Source of truth is the ontology + query + template; this file is not an
// independent second source of truth (GGM-SRC-002).
//
// One entry per declared method with its bounded-status law axis. The vector
// invariant holds: Unknown never collapses into Admitted or Refused.

pub const METHOD_LAW_STATUS: &[(&str, &str)] = &[
    ("max/gateCheck", "CANDIDATE"),
    ("textDocument/codeAction", "CANDIDATE"),
    ("textDocument/diagnostic", "CANDIDATE"),
    ("textDocument/didOpen", "ADMITTED"),
    ("textDocument/hover", "CANDIDATE"),
];
const _: () = ();