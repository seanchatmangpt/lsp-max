//! Tier 2 rule-pack population + did_save court pipeline
//! (agent contract: `rule_pack`, lane WG2).
//!
//! Populates the previously-EMPTY rule pack set (F6 blocker,
//! `lib.rs:57`) with the Tier 2 court pack, and provides the pure
//! classification/diagnostic pipeline that `did_save` executes:
//!
//! source attributes -> per-function classification (seed grid +
//! `#[allow_isel_residual]`) -> ERROR/ANDON_REFUSED diagnostics at the
//! DWARF-mapped span -> live `ReceiptVector`.
//!
//! Verdict law (stated, mechanical):
//! - forward Conditional branch (E9) in a fn with no residual allowance
//!   (attribute or seed grid) -> ERROR `ANDON_REFUSED` at that branch's
//!   DWARF line;
//! - LoopStructural-only branchy fns are admitted (E9 law: a backward
//!   in-function conditional is the shape of a loop, never refused);
//! - branch-free -> clean; seeded/allowed branchy -> residual, clean.
use crate::attributes::{parse_attrs, AttrParseError, Classification, ParsedAttrs, ResidualGrid};
use crate::compiler_worker::{CompileOutcome, CompilerWorker, TargetKind};
use crate::disasm_court::DisasmCourt;
use crate::receipts::ReceiptVector;
use lsp_types_max::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

/// The Tier 2 court rule id (pinned, gate-facing).
pub const ANDON_RULE_ID: &str = "ANDON-REFUSED-001";

/// The Tier 2 pack: one rule declaring the court's verdict law. The regex
/// surface is advisory Tier-1 scope; the AUTHORITATIVE evaluation of this
/// law is the disasm court run in `did_save` (lib.rs), never the regex.
pub fn tier2_pack() -> lsp_max::rule_pack_server::RulePack {
    lsp_max::rule_pack_server::RulePack {
        id: "branchless-tier2@1.0.0".into(),
        version: "1.0.0".into(),
        depends_on: vec![],
        rules: vec![lsp_max_rule("ANDON-REFUSED-001")],
    }
}

/// Build one Rule value without depending on the parent crate's prelude.
fn lsp_max_rule(id: &str) -> lsp_max::rule_pack_server::Rule {
    lsp_max::rule_pack_server::Rule {
        id: id.into(),
        name: "Tier 2 disassembly court refusal".into(),
        severity: "error".into(),
        // Never-matching regex (`\b\B` is a contradiction): the Tier-1
        // regex scan must stay silent — the AUTHORITATIVE evaluation of
        // this rule is the Tier-2 disasm court in did_save (lib.rs), not
        // this surface. An empty pattern would match every character.
        pattern: String::from(r"\b\B"),
        path_globs: vec![],
        exclude_globs: vec![],
        message: "forward conditional branch in branchless-scoped code".into(),
        rationale: "Tier 2 disasm court: forward Conditional edge without residual allowance \
                    (E9 classification)"
            .into(),
        eval_budget: lsp_max::rule_pack_server::EvalBudget::Sync,
    }
}

/// Per-function Tier 2 verdict with the source attributes found for it.
#[derive(Debug, Clone)]
pub struct Tier2FnVerdict {
    /// Function symbol name from the object.
    pub name: String,
    /// Attribute classification (seed grid + parsed attrs).
    pub classification: Classification,
    /// Classified branch sites with DWARF line/col attached (span source).
    pub branch_sites: Vec<crate::disasm_court::BranchSite>,
}

/// Outcome of one Tier 2 court run over one saved document.
#[derive(Debug, Clone, Default)]
pub struct Tier2Outcome {
    /// One verdict per judged function.
    pub functions: Vec<Tier2FnVerdict>,
    /// Live conformance vector.
    pub vector: ReceiptVector,
    /// True when the object carried DWARF line info (else diagnostics fall
    /// back to the 0,0 span and say so).
    pub had_line_map: bool,
    /// Human-readable compile/skip reason when no functions were judged.
    pub note: Option<String>,
}

/// Read per-function attribute flags from source text.
///
/// Mechanical rule: find the fn-definition line (`fn <name>` not followed
/// by an identifier char), then inspect the definition line itself plus the
/// contiguous run of `#[...]` attribute lines immediately above it; an
/// `allow_isel_residual` anywhere in that window sets the flag.
/// Typed attr-window result: `Ok` when every `#[...]` in the definition
/// window parsed, `Err` carrying the typed parse error (fail-closed: the
/// caller refuses and surfaces the error, never launders it to default).
pub fn fn_attr_flags(source: &str, fn_name: &str) -> Result<ParsedAttrs, AttrParseError> {
    let needle = format!("fn {fn_name}");
    let lines: Vec<&str> = source.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(c) = line.find(&needle) else {
            continue;
        };
        let after = &line[c + needle.len()..];
        // Definition site only: next char must not continue an identifier
        // (excludes `fn my_fn_wrapper`, `fn name()` inside patterns...).
        if after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let mut attr_tokens: Vec<String> = Vec::new();
        if let Some(h) = extract_attr_paths(line) {
            attr_tokens.extend(h);
        }
        // Walk the contiguous attribute block above the definition line.
        for prev in lines[..i].iter().rev() {
            let t = prev.trim();
            if t.starts_with("#[") {
                if let Some(h) = extract_attr_paths(prev) {
                    attr_tokens.extend(h);
                }
            } else if !t.is_empty() {
                break;
            }
        }
        // Typed error surfaced, not laundered (LSPA-3 root cause #2): the
        // caller decides the fail-closed fallback AND the error is
        // reported; a bare `unwrap_or_default` here is what previously
        // turned every parameterized allowance into a silent refusal.
        return parse_attrs(&attr_tokens);
    }
    Ok(ParsedAttrs::default())
}

/// Extract attribute path tokens from ALL `#[...]` spans on a line.
/// Returns an empty vec when the line carries no attribute.
fn extract_attr_paths(line: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find("#[") {
        rest = &rest[open + 2..];
        // Bracket-depth scan: payloads may nest brackets (e.g. strings).
        let mut depth = 1usize;
        let mut close = None;
        for (i, c) in rest.char_indices() {
            match c {
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                '[' => depth += 1,
                _ => {}
            }
        }
        let close = close?;
        let inner = &rest[..close];
        tokens.extend(split_attr_inner(inner));
        rest = &rest[close + 1..];
    }
    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

/// Tokenize one `#[...]` inner payload into attribute-path tokens WITHOUT
/// shredding parameterized payloads (LSPA-3 root cause #1).
///
/// Grammar law: a token ends only at whitespace that is OUTSIDE any
/// paren pair and OUTSIDE a string literal. Inside `allow_isel_residual(
/// target = "a, b")` the commas/spaces belong to the payload, so the
/// whole `allow_isel_residual(...)` form arrives at `parse_attrs` as ONE
/// token, whose paren payload `parse_attrs` then splits on top-level
/// commas itself (attributes.rs:129).
///
/// Bare forms are unchanged: `no_mangle` and `allow_isel_residual` split
/// exactly as before because they carry no parens.
fn split_attr_inner(inner: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    let mut in_quote = false;
    let mut escaped = false;
    for c in inner.chars() {
        match c {
            '\\' if in_quote => {
                escaped = true;
                cur.push(c);
            }
            _ if escaped => {
                escaped = false;
                cur.push(c);
            }
            '"' if !escaped => {
                in_quote = !in_quote;
                cur.push(c);
            }
            '(' if !in_quote => {
                depth += 1;
                cur.push(c);
            }
            ')' if !in_quote => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            c if c.is_whitespace() && depth == 0 && !in_quote => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

/// Blank out the branchless-lsp diagnostic-layer attributes
/// (`#[allow_isel_residual(...)]`, `#[branchless_invariant]`) with spaces
/// before handing the source to rustc: rustc refuses unknown attributes
/// (E0438-class), and the attrs are OURS, not rustc's. The replacement is
/// byte-count-preserving so DWARF line/column data stays exact.
pub fn sanitize_for_compile(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(at) = rest.find("#[") {
        // Look ahead: is this an attribute we own?
        let after = &rest[at + 2..];
        let owned = ["allow_isel_residual", "branchless_invariant"]
            .iter()
            .any(|k| {
                after.strip_prefix(k).is_some_and(|tail| {
                    tail.starts_with(']')
                        || tail.starts_with('(')
                        || tail.starts_with(|c: char| c.is_whitespace())
                })
            });
        if !owned {
            // Copy up to and including the '#[' and keep scanning.
            let split = at + 2;
            out.push_str(&rest[..split]);
            rest = &rest[split..];
            continue;
        }
        // Copy everything before `#[`.
        out.push_str(&rest[..at]);
        // Find the matching close bracket (depth-aware).
        let _rest_iter = after.char_indices();
        // Skip the keyword chars we already know.
        let keyword_end = after
            .find([']', '('])
            .expect("owner check guarantees ']' or '('");
        // Scan from after '#[' for the depth-aware close.
        let mut depth = 1usize;
        let mut close = None;
        for (i, c) in after.char_indices().skip(keyword_end) {
            match c {
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                '(' | '[' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        let close = close.expect("owned attribute must close");
        for _ in 0..(after[..=close].len() + 2) {
            out.push(' ');
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Run the full Tier 2 pipeline over one saved source: compile with
/// debuginfo, judge_classified, classify with the seed grid + per-fn
/// attributes, build diagnostics and the live ReceiptVector.
pub fn run_tier2(source: &str, target: TargetKind) -> Tier2Outcome {
    let target_str = match target {
        TargetKind::Aarch64AppleDarwin => "aarch64",
        TargetKind::Riscv32Imc => "riscv32imc",
    };
    let worker = CompilerWorker::new();
    let mut out = Tier2Outcome::default();
    let obj = match worker.compile_debuginfo(&sanitize_for_compile(source), target) {
        CompileOutcome::Object(p) => p,
        CompileOutcome::TargetMissing(e) => {
            out.note = Some(format!("target missing: {e}"));
            return out;
        }
        CompileOutcome::CompileError(e) => {
            out.note = Some(format!("compile error (not a court verdict): {e}"));
            return out;
        }
    };
    let court = DisasmCourt::for_target(&target);
    let grid = ResidualGrid::canonical();
    // LineMap::parse on the bytes we already hold; judge_classified re-reads
    // the file and parses again (court API is path-based).
    let (line_map, fns) = court.judge_classified(&obj);
    out.had_line_map = line_map.row_count() > 0;
    for f in fns {
        // Mach-O prepends `_' to C symbols; source- and grid-keyed lookups
        // use the bare name (the judge's name stays the raw symbol).
        let bare = f.name.trim_start_matches('_');
        // Fail-closed + typed-surfacing (LSPA-3 root cause #2): a parse
        // error falls back to DEFAULT (refusing) attrs AND the typed error
        // is surfaced on the wire as a WARNING note — never a silent
        // unwrap_or_default.
        let attrs = match fn_attr_flags(source, bare) {
            Ok(a) => a,
            Err(e) => {
                let msg = format!("ATTRIBUTE_NOTE (typed parse refusal, fail-closed): {e}");
                out.note = Some(match out.note.take() {
                    Some(prev) => format!("{prev}; {msg}"),
                    None => msg,
                });
                ParsedAttrs::default()
            }
        };
        let _branchy = !f.sites.is_empty();
        // Exact-count court (DOD-15): the bool form `classify_seeded`
        // collapses the count to `usize::from(branchy)` (=1), which admits
        // ANY measured budget >= 1 regardless of the true site count.
        // The wire court must pass the REAL measured count.
        let classification = grid.classify_seeded_count(bare, target_str, &attrs, f.sites.len());
        out.functions.push(Tier2FnVerdict {
            name: bare.to_string(),
            classification,
            branch_sites: f.sites,
        });
    }
    // Fold the class totals from the raw object so n_alu / n_load_store stay
    // exact (judge_classified does not expose per-class counts).
    let raw = court.judge(&obj);
    let report = crate::disasm_court::DisasmReport {
        functions: raw.functions,
    };
    let v = ReceiptVector::fold_seeded(
        &report,
        &ParsedAttrs::default(),
        &grid,
        target_str,
        &mut Vec::new(),
    );
    out.vector.n_alu = v.n_alu;
    out.vector.n_load_store = v.n_load_store;
    // Refusal/residual/admitted counts come from the per-fn classification
    // above (attr-aware), so fold them in explicitly.
    for f in &out.functions {
        match f.classification {
            Classification::Admitted => out.vector.n_admitted += 1,
            Classification::HintResidual => out.vector.n_residual += 1,
            _ => out.vector.n_refused += 1,
        }
    }
    out
}

/// Arch string used for seed-grid target matching.
/// Convert Tier 2 verdicts into LSP diagnostics (ERROR/ANDON_REFUSED at the
/// DWARF-mapped span; DWARF lines are 1-based, LSP ranges are 0-based).
pub fn verdict_diagnostics(out: &Tier2Outcome) -> Vec<Diagnostic> {
    let mut diags: Vec<Diagnostic> = Vec::new();
    // LSPU-2 double-silence fix (lane LSPA-2): a rustc-failure save sets
    // `Tier2Outcome.note` (rule_pack.rs:249-251) but previously published an
    // EMPTY diagnostics array — indistinguishable from a clean save, and the
    // gate counted 0 refusals (false-ADMITTED hazard). Emit the note as a
    // WARNING-class diagnostic (WARNING, not ERROR: rustc failure is a
    // judge-input failure, not an ANDON verdict; the gate keys on
    // severity-ERROR, so this surfaces the silence without a false refusal).
    // Emitted FIRST so ordering is deterministic.
    if let Some(note) = &out.note {
        diags.push(note_diag(note));
    }
    for f in &out.functions {
        let refused = matches!(
            f.classification,
            Classification::ErrorRefused | Classification::ErrorRefusedNoAttr
        );
        if !refused {
            continue;
        }
        if f.branch_sites.is_empty() {
            diags.push(andon_diag(
                f,
                None,
                "ANDON_REFUSED (no DWARF row: branch site unmapped)",
            ));
            continue;
        }
        for s in &f.branch_sites {
            diags.push(andon_diag(f, Some(s), "ANDON_REFUSED"));
        }
    }
    diags
}

/// Rule id for compile-note diagnostics (WARNING class).
pub const COMPILE_NOTE_RULE_ID: &str = "COMPILE-NOTE-001";

/// Max characters of rustc stderr carried in a note diagnostic message.
const NOTE_TRUNCATE: usize = 512;

/// One WARNING-class diagnostic carrying a compile/skip note.
fn note_diag(note: &str) -> Diagnostic {
    // Truncate sanely and deterministically: char-boundary-safe, fixed cap,
    // stable suffix. rustc stderr can be huge; a bounded, byte-stable slice
    // keeps the wire payload deterministic.
    let truncated: String = if note.chars().count() > NOTE_TRUNCATE {
        let mut s: String = note.chars().take(NOTE_TRUNCATE).collect();
        s.push_str("...[truncated]");
        s
    } else {
        note.to_string()
    };
    Diagnostic {
        range: Range::new(Position::new(0, 0), Position::new(0, 1)),
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(COMPILE_NOTE_RULE_ID.into())),
        code_description: None,
        source: Some("branchless-lsp".into()),
        message: format!("COMPILE_NOTE (court not evaluated): {truncated}"),
        related_information: None,
        tags: None,
        data: Some(serde_json::json!({
            "rule": COMPILE_NOTE_RULE_ID,
            "note_class": "compile_or_skip",
            "detail": truncated,
        })),
    }
}

/// One ANDON_REFUSED diagnostic for a fn/site pair.
fn andon_diag(
    f: &Tier2FnVerdict,
    site: Option<&crate::disasm_court::BranchSite>,
    tag: &str,
) -> Diagnostic {
    let (line, col) = site
        .and_then(|s| s.line.map(|l| (l, s.column)))
        .unwrap_or((1, None));
    let line0 = line.saturating_sub(1);
    let col0 = col.unwrap_or(0).saturating_sub(1);
    let range = Range::new(Position::new(line0, col0), Position::new(line0, col0 + 1));
    let where_ = site
        .map(|s| format!("{} {} at line {}", tag, s.mnemonic, line))
        .unwrap_or_else(|| format!("{tag} (no DWARF row)"));
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(ANDON_RULE_ID.into())),
        code_description: None,
        source: Some("branchless-lsp".into()),
        message: format!(
            "ANDON_REFUSED: fn `{}` carries a forward conditional branch; \
             annotate with #[allow_isel_residual] or make it branchless",
            f.name
        ),
        related_information: None,
        tags: None,
        data: Some(serde_json::json!({
            "fn": f.name,
            "classification": format!("{:?}", f.classification),
            "detail": where_,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRANCHY: &str = "#[no_mangle]\npub extern \"C\" fn rf_branchy(x: u32) -> u32 {\n    let mut i = core::hint::black_box(0u32);\n    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }\n    i\n}\n";

    const CLEAN: &str = "#[no_mangle]\npub extern \"C\" fn rf_clean(x: u32) -> u32 {\n    core::hint::black_box(x).wrapping_add(1)\n}\n";

    const RESIDUAL: &str = "#[allow_isel_residual]\n#[no_mangle]\npub extern \"C\" fn rf_allowed(x: u32) -> u32 {\n    let mut i = core::hint::black_box(0u32);\n    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }\n    i\n}\n";

    #[test]
    fn sanitize_blanks_owned_attrs_byte_exactly() {
        let src =
            "#[allow_isel_residual]\n#[no_mangle]\npub extern \"C\" fn t(x: u32) -> u32 { x }";
        let out = sanitize_for_compile(src);
        assert_eq!(out.len(), src.len(), "byte count preserved");
        assert_eq!(
            out.lines().count(),
            src.lines().count(),
            "line count preserved"
        );
        assert!(!out.contains("allow_isel_residual"));
        assert!(out.contains("#[no_mangle]"), "foreign attrs survive");
    }

    #[test]
    fn sanitize_handles_parameterized_form_and_keeps_span() {
        let src = "fn a() {}\n#[allow_isel_residual(target = \"aarch64\", max_branches = 1)]\nfn b(x: u32) -> u32 { x }";
        let out = sanitize_for_compile(src);
        assert_eq!(out.len(), src.len());
        // The fn b line must land on the SAME line number (attr blanked in place).
        let bl = out.lines().nth(2).expect("line 3");
        assert!(bl.contains("fn b"), "fn b line preserved in place");
    }

    #[test]
    fn attr_window_finds_preceding_allowance() {
        let a = fn_attr_flags(RESIDUAL, "rf_allowed").expect("parse ok");
        assert!(
            a.allow_isel_residual,
            "attr directly above the fn line must parse"
        );
        assert!(
            !fn_attr_flags(BRANCHY, "rf_branchy")
                .expect("parse ok")
                .allow_isel_residual,
            "unannotated fn must carry no allowance"
        );
    }

    #[test]
    fn attr_same_line_and_gap_forms_flip() {
        let src = "#[no_mangle] #[allow_isel_residual] pub extern \"C\" fn z(x: u32) -> u32 { x }";
        assert!(
            fn_attr_flags(src, "z")
                .expect("parse ok")
                .allow_isel_residual
        );
        // A fn defined away from any attribute must stay unflagged.
        let far = "fn a() {}\nfn z() {}";
        assert!(
            !fn_attr_flags(far, "z")
                .expect("parse ok")
                .allow_isel_residual
        );
    }

    #[test]
    fn parameterized_payload_survives_extraction_intact() {
        // LSPA-3 root cause #1, anti-vacuity flip: the OLD extractor split
        // on [',', ' '] and shredded this exact payload; the new one must
        // deliver ONE intact token.
        let src = "#[allow_isel_residual(target = \"aarch64\", max_branches = 2, \
                   reason = \"lspa-3 measured budget\")]\nfn f() {}";
        let flags = fn_attr_flags(src, "f").expect("parameterized attr must parse");
        let p = flags
            .residual_params
            .expect("parameterized form must set residual_params");
        assert_eq!(p.target, "aarch64");
        assert_eq!(p.max_branches, Some(2));
        assert_eq!(p.reason.as_deref(), Some("lspa-3 measured budget"));
        assert!(!flags.allow_isel_residual, "bare flag stays unset");
        // Multi-attr line: no_mangle + parameterized co-exist on one line.
        let mixed = "#[no_mangle] #[allow_isel_residual(target = \"any\")] pub fn m() {}";
        let flags = fn_attr_flags(mixed, "m").expect("mixed line parses");
        assert!(
            !flags.allow_isel_residual,
            "no bare allowance on the line: bare flag stays unset"
        );
        assert_eq!(
            flags.residual_params.expect("params").target,
            "any",
            "the parameterized payload must survive the same line as #[no_mangle]"
        );
    }

    #[test]
    fn typed_parse_error_is_surfaced_not_laundered() {
        // Root cause #2, anti-vacuity flip: a malformed parameterized attr
        // must come back as a typed Err (the caller refuses fail-closed and
        // emits the ATTRIBUTE_NOTE), never a silent default.
        let src = "#[allow_isel_residual(wibble = 1)]\nfn f() {}";
        let e = fn_attr_flags(src, "f").expect_err("must be a typed error");
        use crate::attributes::AttrParseErrorKind;
        assert_eq!(e.kind, AttrParseErrorKind::UnknownParamKey);
        assert_eq!(e.attr, "wibble");
    }

    #[test]
    fn tier2_pack_populates_one_error_rule() {
        let p = tier2_pack();
        assert_eq!(p.rules.len(), 1);
        assert_eq!(p.rules[0].id, ANDON_RULE_ID);
        assert_eq!(p.rules[0].severity, "error");
    }

    #[test]
    fn branchy_probe_refused_at_mapped_line() {
        // Real collaborator: full rustc compile + capstone + DWARF.
        let out = run_tier2(BRANCHY, TargetKind::Aarch64AppleDarwin);
        assert!(out.had_line_map, "probe must carry .debug_line");
        let refused: Vec<_> = out
            .functions
            .iter()
            .filter(|f| {
                matches!(
                    f.classification,
                    Classification::ErrorRefused | Classification::ErrorRefusedNoAttr
                )
            })
            .collect();
        assert!(
            refused.iter().any(|f| f.name.contains("rf_branchy")),
            "rf_branchy must be refused; verdicts: {:?}",
            out.functions
        );
        let diags = verdict_diagnostics(&out);
        assert!(
            diags.iter().any(|d| {
                d.severity == Some(DiagnosticSeverity::ERROR)
                    && d.code == Some(NumberOrString::String(ANDON_RULE_ID.into()))
            }),
            ">=1 ERROR/ANDON_REFUSED diagnostic required"
        );
        let mapped = diags
            .iter()
            .find(|d| d.data.as_ref().and_then(|v| v.get("detail")).is_some());
        assert!(mapped.is_some(), "diagnostic must carry a site detail");
    }

    #[test]
    fn clean_probe_admitted_with_zero_error_diags() {
        let out = run_tier2(CLEAN, TargetKind::Aarch64AppleDarwin);
        assert!(
            out.functions.iter().any(
                |f| f.name.contains("rf_clean") && f.classification == Classification::Admitted
            ),
            "rf_clean must admit; verdicts: {:?}",
            out.functions
        );
        assert!(
            verdict_diagnostics(&out).is_empty(),
            "clean file must publish zero diagnostics"
        );
        assert_eq!(out.vector.n_refused, 0);
        assert!(out.vector.n_admitted >= 1);
    }

    #[test]
    fn allowed_residual_is_clean_and_counts_residual() {
        let out = run_tier2(RESIDUAL, TargetKind::Aarch64AppleDarwin);
        let f = out
            .functions
            .iter()
            .find(|f| f.name.contains("rf_allowed"))
            .expect("rf_allowed judged");
        assert_eq!(f.classification, Classification::HintResidual);
        assert!(
            verdict_diagnostics(&out).is_empty(),
            "residual allowance must publish zero diagnostics"
        );
        assert!(out.vector.n_residual >= 1, "vector must count residual");
        assert_eq!(out.vector.n_refused, 0);
    }

    #[test]
    fn seeded_symbol_is_residual_without_annotation() {
        // A fn named after a seed-grid row: branchy but auto-RESIDUAL.
        let src = "#[no_mangle]\npub extern \"C\" fn select_slice(x: u32) -> u32 {\n    let mut i = core::hint::black_box(0u32);\n    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }\n    i\n}\n";
        let out = run_tier2(src, TargetKind::Aarch64AppleDarwin);
        let f = out
            .functions
            .iter()
            .find(|f| f.name.contains("select_slice"))
            .expect("select_slice judged");
        assert_eq!(
            f.classification,
            Classification::HintResidual,
            "seed grid must auto-residual without annotation"
        );
        assert!(verdict_diagnostics(&out).is_empty());
    }

    #[test]
    fn refusal_flips_off_when_allowance_added() {
        // Anti-vacuity: the SAME branchy body with the allowance attr must
        // flip refusal -> residual (a court that cannot flip is FAIL).
        // While-loop body: emits a real forward conditional edge at -O (an
        // `if` may lower to a branchless csel — the court sees the machine
        // code, and correctly admits a csel-only `if`).
        let base = "#[no_mangle]\npub extern \"C\" fn rf_flip(x: u32) -> u32 {\n    let mut i = core::hint::black_box(0u32);\n    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }\n    i\n}\n";
        let allowed = format!("#[allow_isel_residual]\n{base}");
        let refused = run_tier2(base, TargetKind::Aarch64AppleDarwin);
        let residual = run_tier2(&allowed, TargetKind::Aarch64AppleDarwin);
        let rf = refused
            .functions
            .iter()
            .find(|f| f.name.contains("rf_flip"))
            .expect("refused verdict");
        let rr = residual
            .functions
            .iter()
            .find(|f| f.name.contains("rf_flip"))
            .expect("residual verdict");
        assert!(
            matches!(
                rf.classification,
                Classification::ErrorRefused | Classification::ErrorRefusedNoAttr
            ),
            "unannotated branchy must refuse: {:?}",
            rf.classification
        );
        assert_eq!(
            rr.classification,
            Classification::HintResidual,
            "same body + allowance must flip to residual"
        );
        assert!(!verdict_diagnostics(&refused).is_empty());
        assert!(verdict_diagnostics(&residual).is_empty());
    }
}
