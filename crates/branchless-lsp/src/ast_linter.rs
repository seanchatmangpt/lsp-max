//! Tier 1 advisory AST linter (agent contract: `ast_linter`).
//!
//! A `syn::visit` pass over a source string that flags branch-bearing
//! expressions: `ExprIf`, `ExprMatch`, `ExprForLoop`, `ExprWhile`,
//! `ExprLoop`, `ExprTry`, `ExprIndex`, and panic-capable calls
//! (`.unwrap()`, `.expect(...)`, `panic!(...)`, `todo!`, `unimplemented!`).
//!
//! Every hit becomes an `ANDON_BRANCH_SYNTAX` diagnostic candidate. Tier 1
//! is ADVISORY: it never refuses admission — Tier 2 (the disassembly court)
//! is the sole refusing authority. Output feeds `hover` and `code_actions`.

use syn::spanned::Spanned;
use syn::visit::Visit;

/// One branchy-syntax hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchHit {
    /// 1-based line of the expression start.
    pub line: usize,
    /// 1-based column of the expression start.
    pub column: usize,
    /// Classification, e.g. `"if"`, `"match"`, `"unwrap"`.
    pub kind: &'static str,
    /// Diagnostic short code (always `ANDON_BRANCH_SYNTAX` today).
    pub code: &'static str,
}

/// Result of one lint pass.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LintReport {
    /// All hits, in visit order.
    pub hits: Vec<BranchHit>,
}

impl LintReport {
    /// True when the source contains no branchy syntax at all.
    pub fn is_clean(&self) -> bool {
        self.hits.is_empty()
    }
}

/// Lint a Rust source string, returning the advisory hit list.
pub fn lint_source(src: &str) -> LintReport {
    let mut report = LintReport::default();
    let Ok(file) = syn::parse_file(src) else {
        return report;
    };
    // Items carrying #[allow_isel_residual] are DECLARED residual: Tier 1
    // is advisory and must not re-flag syntax the allowance already
    // covers (Tier 2 remains the sole refusing authority).
    let allowed = allow_isel_item_lines(&file);
    let mut visitor = Linter {
        report: &mut report,
        allowed_lines: allowed,
    };
    visitor.visit_file(&file);
    report
}

fn allow_isel_item_lines(file: &syn::File) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for item in &file.items {
        let attrs = match item {
            syn::Item::Const(i) => &i.attrs,
            syn::Item::Static(i) => &i.attrs,
            syn::Item::Fn(i) => &i.attrs,
            _ => continue,
        };
        if attrs
            .iter()
            .any(|a| a.path().is_ident("allow_isel_residual"))
        {
            let sp = item.span();
            v.push((sp.start().line, sp.end().line));
        }
    }
    v
}

struct Linter<'a> {
    report: &'a mut LintReport,
    allowed_lines: Vec<(usize, usize)>,
}

impl<'a> Linter<'a> {
    fn hit(&mut self, span: proc_macro2::Span, kind: &'static str) {
        let lc = span.start();
        if self
            .allowed_lines
            .iter()
            .any(|(a, b)| lc.line >= *a && lc.line <= *b)
        {
            return; // declared residual: allowance covers this item
        }
        self.report.hits.push(BranchHit {
            line: lc.line,
            column: lc.column + 1,
            kind,
            code: "ANDON_BRANCH_SYNTAX",
        });
    }
}

impl<'ast> Visit<'ast> for Linter<'_> {
    fn visit_expr_if(&mut self, e: &'ast syn::ExprIf) {
        self.hit(e.span(), "if");
        syn::visit::visit_expr_if(self, e);
    }
    fn visit_expr_match(&mut self, e: &'ast syn::ExprMatch) {
        self.hit(e.span(), "match");
        syn::visit::visit_expr_match(self, e);
    }
    fn visit_expr_for_loop(&mut self, e: &'ast syn::ExprForLoop) {
        self.hit(e.span(), "for");
        syn::visit::visit_expr_for_loop(self, e);
    }
    fn visit_expr_while(&mut self, e: &'ast syn::ExprWhile) {
        self.hit(e.span(), "while");
        syn::visit::visit_expr_while(self, e);
    }
    fn visit_expr_loop(&mut self, e: &'ast syn::ExprLoop) {
        self.hit(e.span(), "loop");
        syn::visit::visit_expr_loop(self, e);
    }
    fn visit_expr_try(&mut self, e: &'ast syn::ExprTry) {
        self.hit(e.span(), "try");
        syn::visit::visit_expr_try(self, e);
    }
    fn visit_expr_index(&mut self, e: &'ast syn::ExprIndex) {
        self.hit(e.span(), "index");
        syn::visit::visit_expr_index(self, e);
    }
    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        match e.method.to_string().as_str() {
            "unwrap" | "expect" => self.hit(e.span(), e.method.to_string().leak()),
            _ => {}
        }
        syn::visit::visit_expr_method_call(self, e);
    }
    fn visit_macro(&mut self, m: &syn::Macro) {
        let name = m.path.segments.last().map(|s| s.ident.to_string());
        if let Some(k) = name.as_deref().and_then(|n| match n {
            "panic" | "todo" | "unimplemented" | "unreachable" => Some("panic-macro"),
            _ => None,
        }) {
            // Macros carry no Expr receiver; synthesize a synthetic 0/0 hit.
            self.report.hits.push(BranchHit {
                line: 0,
                column: 0,
                kind: k,
                code: "ANDON_BRANCH_SYNTAX",
            });
        }
        syn::visit::visit_macro(self, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_source_has_zero_hits() {
        let r = lint_source("fn f() -> u32 { 41 + 1 }");
        assert!(r.is_clean());
    }

    #[test]
    fn if_and_unwrap_are_flagged() {
        let r =
            lint_source("fn f(x: Option<u32>) -> u32 { if x.is_some() { x.unwrap() } else { 0 } }");
        let kinds: Vec<_> = r.hits.iter().map(|h| h.kind).collect();
        assert!(kinds.contains(&"if"), "expected if hit, got {kinds:?}");
        assert!(
            kinds.contains(&"unwrap"),
            "expected unwrap hit, got {kinds:?}"
        );
        assert!(r.hits.iter().all(|h| h.code == "ANDON_BRANCH_SYNTAX"));
    }

    #[test]
    fn report_is_advisory_only_shape() {
        // Tier 1 carries no refusal field — that is Tier 2's authority.
        let r = lint_source("fn f() { match 1 { _ => {} } }");
        assert_eq!(r.hits.len(), 1);
        assert_eq!(r.hits[0].kind, "match");
    }
}
