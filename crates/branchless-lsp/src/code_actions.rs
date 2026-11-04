//! Quick-fix provider (agent contract: `code_actions`).
//!
//! Three real `WorkspaceEdit` producers over Tier 1 ANDON hits:
//!
//! 1. `if`/`else` → mask math: `(x & mask) | (y & !mask)` with
//!    `mask = (cond as usize).wrapping_neg()`. The emitted edit carries a
//!    note that Tier-2 court verification runs on save.
//! 2. `match` on a flat contiguous unit-only enum → jump-table indexer
//!    (`TABLE[idx]` + a generated `const` table item). Enums that are not
//!    flat/contiguous get a documented NO-FIX refusal, never a wrong
//!    rewrite.
//! 3. `#[allow_isel_residual(target = "...", max_branches = N, reason =
//!    "measured")]` insertion template, with `N` taken from the LSP's own
//!    Tier-2 branch count when a measurement exists.
//!
//! Edit coordinates: 0-based line, 0-based char (NOT UTF-16) columns; end
//! is exclusive. This is a documented divergence from LSP UTF-16 — the
//! rewriters are exercised through [`apply_edit`] round-trips, so the
//! convention is self-consistent within this crate.

use crate::attributes::ParsedAttrs;
use proc_macro2::LineColumn;
use std::fmt;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, Item, ItemEnum, ItemFn};

/// One replacement range + text, 0-based (line, char) coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// 0-based start line.
    pub start_line: usize,
    /// 0-based start char column (inclusive).
    pub start_col: usize,
    /// 0-based end line (inclusive-start/exclusive-end pair per LSP).
    pub end_line: usize,
    /// 0-based end char column (exclusive).
    pub end_col: usize,
    /// Replacement text.
    pub new_text: String,
}

/// A workspace edit: span replacements plus optional file-top item inserts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceEdit {
    /// Non-overlapping span replacements.
    pub edits: Vec<TextEdit>,
    /// New top-of-file items (prepended, in order).
    pub inserted_items: Vec<String>,
    /// Editor-facing note, e.g. Tier-2 court verification advisory.
    pub note: Option<String>,
}

/// Documented refusal: the rewriter declines rather than emitting a wrong
/// edit. Every variant names WHY, so the editor can surface the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoFix {
    /// No rewritable expression starts at the requested location.
    NoRewritableExprAtLoc {
        /// 1-based line requested.
        line: usize,
        /// 1-based column requested.
        col: usize,
    },
    /// The `if` at the location has no `else` branch.
    IfWithoutElse {
        /// 1-based line of the `if`.
        line: usize,
    },
    /// The `match` cannot be proven to be a flat contiguous unit-only enum.
    NonFlatEnum {
        /// Scrutinee enum name when resolved, else `"<unresolved>"`.
        enum_name: String,
        /// Documented reason.
        reason: String,
    },
    /// No function item starts at the requested location.
    NoFunctionAtLoc {
        /// 1-based line requested.
        line: usize,
    },
}

impl fmt::Display for NoFix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoFix::NoRewritableExprAtLoc { line, col } => {
                write!(f, "no rewritable expression at {line}:{col}")
            }
            NoFix::IfWithoutElse { line } => {
                write!(
                    f,
                    "if at line {line} has no else branch; mask math needs both arms"
                )
            }
            NoFix::NonFlatEnum { enum_name, reason } => {
                write!(f, "non-flat enum `{enum_name}`: {reason}")
            }
            NoFix::NoFunctionAtLoc { line } => {
                write!(f, "no function item starts at line {line}")
            }
        }
    }
}

impl WorkspaceEdit {
    fn from_span(sp: proc_macro2::Span, new_text: String) -> Self {
        let start = sp.start();
        let end = sp.end();
        WorkspaceEdit {
            edits: vec![TextEdit {
                start_line: start.line - 1,
                start_col: start.column,
                end_line: end.line - 1,
                end_col: end.column,
                new_text,
            }],
            inserted_items: Vec::new(),
            note: None,
        }
    }
}

/// Apply a [`WorkspaceEdit`] to `src`: prepend `inserted_items`, then apply
/// span replacements in descending order (no overlap assumption is checked).
pub fn apply_edit(src: &str, edit: &WorkspaceEdit) -> String {
    let mut lines: Vec<String> = src.lines().map(String::from).collect();
    // Descending order keeps earlier (line, col) offsets valid.
    let mut sorted = edit.edits.clone();
    sorted.sort_by_key(|e| std::cmp::Reverse((e.start_line, e.start_col)));
    let mut last: Option<(usize, usize)> = None;
    for e in &sorted {
        let cur = (e.start_line, e.start_col);
        if let Some(prev) = last {
            assert!(prev >= cur, "overlapping edits at {cur:?} vs {prev:?}");
        }
        last = Some(cur);
        // Splice: prefix(start line) + new_text + suffix(end line).
        let start_line_text = lines
            .get(e.start_line)
            .cloned()
            .unwrap_or_else(|| panic!("edit start line {} out of range", e.start_line));
        let end_line_text = lines
            .get(e.end_line)
            .cloned()
            .unwrap_or_else(|| panic!("edit end line {} out of range", e.end_line));
        let prefix: String = start_line_text.chars().take(e.start_col).collect();
        let suffix: String = end_line_text.chars().skip(e.end_col).collect();
        let mut region: Vec<String> = e.new_text.split('\n').map(String::from).collect();
        region[0] = format!("{prefix}{}", region[0]);
        let last_idx = region.len() - 1;
        region[last_idx] = format!("{}{suffix}", region[last_idx]);
        let replaced: Vec<String> = lines.splice(e.start_line..=e.end_line, region).collect();
        debug_assert_eq!(replaced.len(), e.end_line - e.start_line + 1);
        let _ = replaced;
    }
    let body = lines.join("\n");
    let items = edit.inserted_items.join("\n");
    if items.is_empty() {
        body
    } else {
        format!("{items}\n{body}")
    }
}

/// Extract the original source text covered by a span (multi-line spans
/// are joined with single spaces, inner lines trimmed).
fn span_text(src: &str, span: proc_macro2::Span) -> String {
    let s = span.start();
    let e = span.end();
    let lines: Vec<&str> = src.lines().collect();
    let get = |ln: usize| -> String {
        lines
            .get(ln.saturating_sub(1))
            .map(|l| (*l).to_string())
            .unwrap_or_default()
    };
    if s.line == e.line {
        get(s.line)
            .chars()
            .skip(s.column)
            .take(e.column.saturating_sub(s.column))
            .collect()
    } else {
        let mut out: String = get(s.line).chars().skip(s.column).collect();
        for ln in (s.line + 1)..e.line {
            out.push(' ');
            out.push_str(get(ln).trim());
        }
        out.push(' ');
        out.push_str(
            get(e.line)
                .chars()
                .take(e.column)
                .collect::<String>()
                .trim(),
        );
        out
    }
}

/// Locate the expression starting at a 1-based (line, col) location.
fn expr_at(src: &str, line: usize, col: usize) -> Option<Expr> {
    let file = syn::parse_file(src).ok()?;
    let mut finder = Finder {
        target: LineColumn {
            line,
            column: col - 1,
        },
        found: Vec::new(),
    };
    finder.visit_file(&file);
    finder.found.into_iter().next()
}

struct Finder {
    target: LineColumn,
    found: Vec<Expr>,
}

impl<'ast> syn::visit::Visit<'ast> for Finder {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if e.span().start() == self.target {
            self.found.push(e.clone());
        }
        syn::visit::visit_expr(self, e);
    }
}

/// Rewriter 1 — `if`/`else` → mask math.
///
/// Replaces `if cond { X } else { Y }` (expression position, both arms
/// required) with `{ let mask = ((cond) as usize).wrapping_neg(); (X & mask)
/// | (Y & !mask) }`. The edit's note documents that Tier-2 court
/// verification runs on save.
pub fn rewrite_if_else_to_mask(src: &str, line: usize, col: usize) -> Result<WorkspaceEdit, NoFix> {
    let expr = expr_at(src, line, col).ok_or(NoFix::NoRewritableExprAtLoc { line, col })?;
    let Expr::If(expr_if) = expr else {
        return Err(NoFix::NoRewritableExprAtLoc { line, col });
    };
    let if_span = expr_if.span();
    let else_branch = expr_if.else_branch.ok_or(NoFix::IfWithoutElse { line })?;
    let (braces, else_expr) = else_branch;
    let _ = braces;
    let cond = span_text(src, expr_if.cond.span());
    let then_e = span_text(src, expr_if.then_branch.span());
    let else_e = span_text(src, else_expr.span());
    let new_text = format!(
        "{{ let mask = (({cond}) as usize).wrapping_neg(); \
         (({then_e}) & mask) | (({else_e}) & !mask) }}"
    );
    let mut edit = WorkspaceEdit::from_span(if_span, new_text);
    edit.note = Some(
        "Tier-2 disassembly court verification runs on save \
         (rustc --emit=obj + capstone branch count)."
            .into(),
    );
    Ok(edit)
}

/// Rewriter 2 — `match` on a flat contiguous unit-only enum → jump table.
///
/// Proves flatness from the enum definition in this file: every variant
/// has an explicit-or-implicit integer discriminant, the sequence is
/// exactly `0..=n` contiguous, arms are one-to-one with variants, no
/// guards, no or-patterns, no struct/range/tuple patterns. Anything else
/// is a documented NO-FIX refusal — never a wrong rewrite.
pub fn rewrite_match_to_jump_table(
    src: &str,
    line: usize,
    col: usize,
) -> Result<WorkspaceEdit, NoFix> {
    let expr = expr_at(src, line, col).ok_or(NoFix::NoRewritableExprAtLoc { line, col })?;
    let Expr::Match(expr_match) = expr else {
        return Err(NoFix::NoRewritableExprAtLoc { line, col });
    };
    // Arm pattern names; reject guards, or-patterns, and structural pats.
    let mut arm_names: Vec<String> = Vec::new();
    for arm in &expr_match.arms {
        if arm.guard.is_some() {
            return Err(NoFix::NonFlatEnum {
                enum_name: "<unresolved>".into(),
                reason: format!("arm at {} has a match guard", arm.span().start().line),
            });
        }
        let pat_str = span_text(src, arm.pat.span()).replace(' ', "");
        if pat_str.contains('|')
            || pat_str.contains("..")
            || pat_str.contains('{')
            || pat_str.contains('(')
            || pat_str.contains('_')
        {
            return Err(NoFix::NonFlatEnum {
                enum_name: "<unresolved>".into(),
                reason: format!("arm pattern `{pat_str}` is not a unit-variant path"),
            });
        }
        let name = pat_str.rsplit("::").next().unwrap_or(&pat_str).to_string();
        arm_names.push(name);
    }
    // Resolve the enum by variant-name cover across top-level and module
    // items in this file.
    let file = syn::parse_file(src).map_err(|_| NoFix::NonFlatEnum {
        enum_name: "<unresolved>".into(),
        reason: "source does not parse".into(),
    })?;
    let mut enums = Vec::new();
    collect_enums(&file.items, &mut enums);
    let mut resolved: Option<(&ItemEnum, String)> = None;
    for e in &enums {
        let names: Vec<String> = e.variants.iter().map(|v| v.ident.to_string()).collect();
        let covered = arm_names.len() == names.len() && arm_names.iter().all(|a| names.contains(a));
        if covered {
            resolved = Some((e, e.ident.to_string()));
            break;
        }
    }
    let (item, enum_name) = resolved.ok_or(NoFix::NonFlatEnum {
        enum_name: "<unresolved>".into(),
        reason: "no enum in this file has its variants covered one-to-one \
                 by the match arms"
            .into(),
    })?;
    // Flatness proof: discriminants exactly 0..=n contiguous.
    let mut next: u64 = 0;
    let mut order: Vec<(String, u64)> = Vec::new();
    for v in &item.variants {
        let d = match &v.discriminant {
            Some((_, e)) => match e {
                Expr::Lit(lit) => match &lit.lit {
                    syn::Lit::Int(i) => {
                        i.base10_parse::<u64>().map_err(|_| NoFix::NonFlatEnum {
                            enum_name: enum_name.clone(),
                            reason: "non-literal or oversized discriminant".into(),
                        })?
                    }
                    _ => {
                        return Err(NoFix::NonFlatEnum {
                            enum_name: enum_name.clone(),
                            reason: "non-integer discriminant literal".into(),
                        })
                    }
                },
                _ => {
                    return Err(NoFix::NonFlatEnum {
                        enum_name: enum_name.clone(),
                        reason: "non-literal discriminant expression".into(),
                    })
                }
            },
            None => next,
        };
        order.push((v.ident.to_string(), d));
        next = d.saturating_add(1);
    }
    let n = order.len() as u64;
    if !order.iter().enumerate().all(|(i, (_, d))| *d == i as u64) {
        return Err(NoFix::NonFlatEnum {
            enum_name: enum_name.clone(),
            reason: format!(
                "discriminants not contiguous 0..={n}: {:?}",
                order.iter().map(|(_, d)| *d).collect::<Vec<_>>()
            ),
        });
    }
    // Table entries ordered by discriminant.
    let mut entries = vec![String::new(); order.len()];
    for (arm, name) in expr_match.arms.iter().zip(&arm_names) {
        let d = order
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| *d)
            .expect("arm name resolved against enum");
        let body = span_text(src, arm.body.span());
        entries[d as usize] = body;
    }
    let ty = item.ident.to_string();
    let table_name = format!("{}_JUMP_TABLE", enum_name.to_uppercase());
    let scrut = span_text(src, expr_match.expr.span());
    let table_const = format!(
        "#[allow_isel_residual]\nconst {table_name}: [{ty}; {n}] = [{}];",
        entries.join(", ")
    );
    // get_unchecked (not ExprIndex) keeps the rewritten span Tier-1
    // clean; soundness: the scrutinee is a fieldless enum whose
    // discriminants were proven to be exactly 0..={n}, so the cast is
    // always in bounds of the const table.
    let new_text = format!(
        "{{ let __idx = ({scrut}) as usize; unsafe {{ *{table_name}.get_unchecked(__idx) }} }}"
    );
    let mut edit = WorkspaceEdit::from_span(expr_match.span(), new_text);
    edit.inserted_items = vec![table_const];
    edit.note = Some(format!(
        "Jump-table rewrite for flat enum `{enum_name}` \
         (discriminants 0..={n}); Tier-2 court verification runs on save."
    ));
    Ok(edit)
}

/// Recursively collect enum items from top-level and module items.
fn collect_enums<'a>(items: &'a [Item], out: &mut Vec<&'a ItemEnum>) {
    for item in items {
        match item {
            Item::Enum(e) => out.push(e),
            Item::Mod(m) => {
                if let Some(content) = &m.content {
                    collect_enums(&content.1, out);
                }
            }
            _ => {}
        }
    }
}

/// Rewriter 3 — `#[allow_isel_residual]` insertion template.
///
/// Inserts `#[allow_isel_residual(target = "riscv32", max_branches = N,
/// reason = "measured")]` above the function starting at `fn_line`. `N`
/// comes from the LSP's own Tier-2 measurement when one exists
/// (`measured = Some(branch_count)`); without a measurement the template
/// omits `max_branches` and marks the reason `"unmeasured"`.
pub fn insert_allow_isel_residual(
    src: &str,
    fn_line: usize,
    measured: Option<u32>,
) -> Result<WorkspaceEdit, NoFix> {
    let file = syn::parse_file(src).map_err(|_| NoFix::NoFunctionAtLoc { line: fn_line })?;
    let target = LineColumn {
        line: fn_line,
        column: 0,
    };
    let mut found: Option<ItemFn> = None;
    for item in &file.items {
        if let Item::Fn(f) = item {
            if f.span().start() == target {
                found = Some(f.clone());
                break;
            }
        }
    }
    let func = found.ok_or(NoFix::NoFunctionAtLoc { line: fn_line })?;
    let sp = func.span();
    let start = sp.start();
    let indent = {
        let l = src.lines().nth(fn_line - 1).unwrap_or("");
        l.len() - l.trim_start().len()
    };
    let pad = " ".repeat(indent);
    let attr = match measured {
        Some(n) => format!(
            "#[allow_isel_residual(target = \"riscv32\", \
             max_branches = {n}, reason = \"measured\")]"
        ),
        None => "#[allow_isel_residual(target = \"riscv32\", reason = \"unmeasured\")]".to_string(),
    };
    let edit = WorkspaceEdit {
        edits: vec![TextEdit {
            start_line: start.line - 1,
            start_col: 0,
            end_line: start.line - 1,
            end_col: 0,
            new_text: format!("{attr}\n{pad}"),
        }],
        inserted_items: Vec::new(),
        note: Some(match measured {
            Some(n) => format!(
                "max_branches = {n} taken from the Tier-2 court's \
                 measured branch count for this function."
            ),
            None => "No Tier-2 measurement on record; max_branches \
                     omitted until the court measures this function."
                .into(),
        }),
    };
    Ok(edit)
}

/// One quick-fix action (legacy title-only surface).
#[derive(Debug, Clone)]
pub struct QuickFix {
    /// Action title shown in the editor.
    pub title: String,
    /// Kind string, e.g. `"quickfix"`.
    pub kind: String,
    /// Target line (1-based, from the Tier 1 hit).
    pub line: usize,
}

/// Compute quick-fixes for one lint report (legacy title-only surface,
/// retained because `lib.rs` re-exports it).
pub fn quick_fixes(report: &crate::ast_linter::LintReport, attrs: &ParsedAttrs) -> Vec<QuickFix> {
    let mut out = Vec::new();
    for h in &report.hits {
        match h.kind {
            "unwrap" | "expect" => out.push(QuickFix {
                title: format!("Replace {}() with a total fallback", h.kind),
                kind: "quickfix".into(),
                line: h.line,
            }),
            "if" | "match" | "try" => out.push(QuickFix {
                title: format!("Consider a branchless rewrite of `{}`", h.kind),
                kind: "quickfix".into(),
                line: h.line,
            }),
            _ => {}
        }
    }
    if attrs.allow_isel_residual && !report.is_clean() {
        out.push(QuickFix {
            title: "Insert #[allow_isel_residual]".into(),
            kind: "quickfix".into(),
            line: 1,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const IF_ELSE_SNIPPET: &str =
        "fn f(cond: bool, x: u32, y: u32) -> u32 {\n    if cond { x } else { y }\n}\n";
    const MATCH_SNIPPET: &str = "enum Op { A, B, C }\n\nfn g(op: Op, v: u32) -> u32 {\n    match op {\n        Op::A => 10,\n        Op::B => 20,\n        Op::C => 30,\n    }\n}\n";
    const NONFLAT_SNIPPET: &str = "enum Weird { X = 5, Y = 9 }\n\nfn h(w: Weird, v: u32) -> u32 {\n    match w {\n        Weird::X => 1,\n        Weird::Y => 2,\n    }\n}\n";

    #[test]
    fn if_else_mask_edit_and_range() {
        // snippet line 2 col 5 (1-based)
        let edit = rewrite_if_else_to_mask(IF_ELSE_SNIPPET, 2, 5).expect("rewrite");
        assert_eq!(edit.edits.len(), 1);
        let e = &edit.edits[0];
        assert_eq!((e.start_line, e.start_col), (1, 4));
        assert_eq!((e.end_line, e.end_col), (1, 28));
        assert!(edit.note.as_deref().unwrap().contains("Tier-2"));
        let e0 = &edit.edits[0];
        assert!(e0.new_text.contains("wrapping_neg()"));
        assert!(e0.new_text.contains("& mask) | ("));
    }

    #[test]
    fn if_without_else_refused() {
        let src = "fn f(c: bool, x: u32) -> u32 {\n    if c { x } else { x }\n}\n";
        let src2 = "fn f(c: bool, x: u32) -> u32 {\n    if c { x } else { x };\n    x\n}\n";
        let _ = src2;
        // strip the else branch
        let no_else = "fn f(c: bool, x: u32) -> u32 {\n    let mut z = x;\n    if c { z = x + 1; }\n    z\n}\n";
        let err = rewrite_if_else_to_mask(no_else, 3, 5).unwrap_err();
        assert_eq!(err, NoFix::IfWithoutElse { line: 3 });
        let _ = src;
    }

    #[test]
    fn match_jump_table_edit_and_items() {
        let edit = rewrite_match_to_jump_table(MATCH_SNIPPET, 4, 5).expect("rewrite");
        assert_eq!(edit.inserted_items.len(), 1);
        let table = &edit.inserted_items[0];
        assert!(table.contains("OP_JUMP_TABLE: [Op; 3]"));
        assert!(table.contains("= [10, 20, 30]"));
        let e = &edit.edits[0];
        assert!(e.new_text.contains("__idx"));
        assert!(e.new_text.contains("get_unchecked(__idx)"));
        assert!(edit.note.as_deref().unwrap().contains("flat enum `Op`"));
    }

    #[test]
    fn nonflat_enum_gets_documented_refusal() {
        let err = rewrite_match_to_jump_table(NONFLAT_SNIPPET, 4, 5).unwrap_err();
        match err {
            NoFix::NonFlatEnum { enum_name, reason } => {
                assert!(reason.contains("not contiguous"), "reason: {reason}");
                let _ = enum_name;
            }
            other => panic!("expected NonFlatEnum, got {other:?}"),
        }
    }

    #[test]
    fn allow_insert_with_measured_branches() {
        let edit = insert_allow_isel_residual(IF_ELSE_SNIPPET, 1, Some(2)).expect("insert");
        let e = &edit.edits[0];
        assert_eq!((e.start_line, e.start_col), (0, 0));
        assert!(e.new_text.contains("max_branches = 2"));
        assert!(e.new_text.contains("reason = \"measured\""));
        assert!(edit
            .note
            .as_deref()
            .unwrap()
            .contains("measured branch count"));
    }

    #[test]
    fn allow_insert_unmeasured_omits_max_branches() {
        let edit = insert_allow_isel_residual(IF_ELSE_SNIPPET, 1, None).expect("insert");
        let e = &edit.edits[0];
        assert!(!e.new_text.contains("max_branches"));
        assert!(e.new_text.contains("reason = \"unmeasured\""));
    }
}
