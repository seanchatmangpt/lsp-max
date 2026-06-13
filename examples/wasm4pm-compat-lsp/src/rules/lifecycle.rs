use serde::{Deserialize, Serialize};
use syn::visit::{self, Visit};

/// Severity level for the LSP diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// A diagnostic item emitted by the linter/LSP engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

fn evd_direct_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"\bEvidence\s*(::\s*raw\b|<[^>]*\b(Raw|Parsed)\b[^>]*>).*?\.(project|export|into_exportable|into_projected|into_receipted)\("
        ).unwrap()
    })
}

fn evd_type_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"\b(RawOcelEvidence|RawXesEvidence|Evidence<.*,\s*(Raw|Parsed)\s*,.*>).*?\.(project|export|into_exportable|into_projected|into_receipted)\("
        ).unwrap()
    })
}

fn fmt_names_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"\b(ocel_to_xes|xes_to_ocel|xes_to_oced|ocel_flatten_to_xes|xes_enrich_to_oced)\b",
        )
        .unwrap()
    })
}

/// Checks the source code using regex patterns for rapid diagnostics.
pub fn check_source_regex(source: &str) -> Vec<LspDiagnostic> {
    let mut diagnostics = Vec::new();

    let evd_direct = evd_direct_re();
    let evd_type = evd_type_re();
    let fmt_names = fmt_names_re();

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1;

        if evd_direct.is_match(line) || evd_type.is_match(line) {
            diagnostics.push(LspDiagnostic {
                code: "W4PM-EVD".to_string(),
                message: "Evidence Lifecycle Misuse (W4PM-EVD): Raw/Parsed evidence cannot be exported, projected, or transitioned directly. Route it through an Admit implementation first.".to_string(),
                severity: DiagnosticSeverity::Error,
                start_line: line_num,
                start_col: 1,
                end_line: line_num,
                end_col: line.len() + 1,
            });
        }

        if fmt_names.is_match(line) {
            diagnostics.push(LspDiagnostic {
                code: "W4PM-FMT".to_string(),
                message: "Format Laundering (W4PM-FMT): Direct format-to-format conversion detected. Ensure all transformations use Admit and Project with a LossReport.".to_string(),
                severity: DiagnosticSeverity::Error,
                start_line: line_num,
                start_col: 1,
                end_line: line_num,
                end_col: line.len() + 1,
            });
        }
    }

    diagnostics
}

/// Visitor that analyzes the AST structure of Rust source code for rule violations.
struct LifecycleVisitor {
    diagnostics: Vec<LspDiagnostic>,
    raw_variables: std::collections::HashSet<String>,
}

impl<'ast> Visit<'ast> for LifecycleVisitor {
    fn visit_local(&mut self, local: &'ast syn::Local) {
        let mut is_raw = false;

        // 1. Check type annotations
        if let syn::Pat::Type(pat_type) = &local.pat {
            let type_str = quote::quote!(#pat_type.ty).to_string();
            if type_str.contains("Raw")
                || type_str.contains("Parsed")
                || type_str.contains("RawOcel")
                || type_str.contains("RawXes")
            {
                is_raw = true;
            }
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                let name = pat_ident.ident.to_string();
                if is_raw {
                    self.raw_variables.insert(name);
                }
            }
        }

        // 2. Check variable names or initializer signatures
        if let syn::Pat::Ident(pat_ident) = &local.pat {
            let name = pat_ident.ident.to_string();
            if name.starts_with("raw_") || name.starts_with("parsed_") {
                self.raw_variables.insert(name.clone());
            }
            if let Some(init) = &local.init {
                let expr = &init.expr;
                let init_str = quote::quote!(#expr).to_string();
                if init_str.contains("Evidence::raw")
                    || init_str.contains("Raw")
                    || init_str.contains("into_parsed")
                {
                    self.raw_variables.insert(name);
                }
            }
        }

        visit::visit_local(self, local);
    }

    fn visit_expr_method_call(&mut self, method_call: &'ast syn::ExprMethodCall) {
        let method_name = method_call.method.to_string();
        if [
            "project",
            "export",
            "into_projected",
            "into_exportable",
            "into_receipted",
        ]
        .contains(&method_name.as_str())
        {
            let mut is_violating = false;

            match &*method_call.receiver {
                syn::Expr::Path(expr_path) => {
                    let path_str = quote::quote!(#expr_path).to_string();
                    if self.raw_variables.contains(&path_str)
                        || path_str.starts_with("raw_")
                        || path_str.starts_with("parsed_")
                    {
                        is_violating = true;
                    }
                }
                other => {
                    let expr_str = quote::quote!(#other).to_string();
                    if expr_str.contains("Evidence::raw")
                        || expr_str.contains("Raw")
                        || expr_str.contains("into_parsed")
                    {
                        is_violating = true;
                    }
                }
            }

            if is_violating {
                let span = method_call.method.span();
                let start = span.start();
                let end = span.end();
                self.diagnostics.push(LspDiagnostic {
                    code: "W4PM-EVD".to_string(),
                    message: format!(
                        "Evidence Lifecycle Misuse (W4PM-EVD): Attempted to call `{}` directly on unadmitted Raw/Parsed evidence. Route it through Admit first.",
                        method_name
                    ),
                    severity: DiagnosticSeverity::Error,
                    start_line: start.line,
                    start_col: start.column + 1,
                    end_line: end.line,
                    end_col: end.column + 1,
                });
            }
        }

        visit::visit_expr_method_call(self, method_call);
    }

    fn visit_item_fn(&mut self, item_fn: &'ast syn::ItemFn) {
        let span = item_fn.sig.ident.span();
        self.check_fn_signature(&item_fn.sig, span);
        visit::visit_item_fn(self, item_fn);
    }

    fn visit_impl_item_fn(&mut self, impl_item_fn: &'ast syn::ImplItemFn) {
        let span = impl_item_fn.sig.ident.span();
        self.check_fn_signature(&impl_item_fn.sig, span);
        visit::visit_impl_item_fn(self, impl_item_fn);
    }
}

impl LifecycleVisitor {
    fn check_fn_signature(&mut self, sig: &syn::Signature, span: proc_macro2::Span) {
        let sig_str = quote::quote!(#sig).to_string();
        let has_ocel = sig_str.contains("Ocel") || sig_str.contains("ocel");
        let has_xes = sig_str.contains("Xes")
            || sig_str.contains("xes")
            || sig_str.contains("Oced")
            || sig_str.contains("oced");

        if has_ocel && has_xes {
            let has_loss_report = sig_str.contains("LossReport")
                || sig_str.contains("LossPolicy")
                || sig_str.contains("Project");
            let fn_name = sig.ident.to_string();
            let matches_laundering_name = fn_name == "ocel_to_xes"
                || fn_name == "xes_to_ocel"
                || fn_name == "xes_to_oced"
                || fn_name == "ocel_flatten_to_xes"
                || fn_name == "xes_enrich_to_oced";

            if matches_laundering_name || !has_loss_report {
                let start = span.start();
                let end = span.end();
                self.diagnostics.push(LspDiagnostic {
                    code: "W4PM-FMT".to_string(),
                    message: format!(
                        "Format Laundering (W4PM-FMT): Function `{}` converts format structures directly without admission and named projection using a LossReport.",
                        fn_name
                    ),
                    severity: DiagnosticSeverity::Error,
                    start_line: start.line,
                    start_col: start.column + 1,
                    end_line: end.line,
                    end_col: end.column + 1,
                });
            }
        }
    }
}

/// Checks the source code using the syn AST parser.
pub fn check_source_ast(source: &str) -> Result<Vec<LspDiagnostic>, syn::Error> {
    let syntax = syn::parse_str::<syn::File>(source)?;
    let mut visitor = LifecycleVisitor {
        diagnostics: Vec::new(),
        raw_variables: std::collections::HashSet::new(),
    };
    visitor.visit_file(&syntax);
    Ok(visitor.diagnostics)
}

/// Runs both regex and AST-based matching rules on the Rust source, combining and deduplicating results.
pub fn check_source(source: &str) -> Vec<LspDiagnostic> {
    let mut diagnostics = check_source_regex(source);
    if let Ok(ast_diagnostics) = check_source_ast(source) {
        diagnostics.extend(ast_diagnostics);
    }

    // Deduplicate diagnostics based on code, line, and message to keep feedback clear.
    let mut unique = std::collections::HashSet::new();
    diagnostics.retain(|d| {
        let key = (d.code.clone(), d.start_line, d.message.clone());
        unique.insert(key)
    });

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_evidence_usage() {
        let code = r#"
            fn process_evidence() {
                let raw_ev = Evidence::<_, Raw, Ocel20>::raw(b"log-bytes");
                let admitted = MyAdmitter::admit(raw_ev).unwrap();
                let projected = admitted.into_projected();
                let exportable = projected.into_exportable();
                let exported = MyExporter::export(&exportable, LossPolicy::RefuseLoss);
            }
        "#;
        let diags = check_source(code);
        // Should have zero errors because it follows Admitted -> Projected -> Exportable path correctly.
        assert!(diags.is_empty(), "Expected no violations, got: {:?}", diags);
    }

    #[test]
    fn test_raw_projection_misuse() {
        let code = r#"
            fn bad_projection() {
                let raw_ev = Evidence::<_, Raw, Ocel20>::raw(b"log-bytes");
                let projected = raw_ev.into_projected(); // VIOLATION (W4PM-EVD)
            }
        "#;
        let diags = check_source(code);
        let evd_violations: Vec<_> = diags.iter().filter(|d| d.code == "W4PM-EVD").collect();
        assert!(
            !evd_violations.is_empty(),
            "Should detect direct projection of Raw evidence"
        );
    }

    #[test]
    fn test_raw_export_misuse() {
        let code = r#"
            fn bad_export() {
                let raw_ev = Evidence::<_, Raw, Ocel20>::raw(b"log-bytes");
                let exported = raw_ev.export(LossPolicy::RefuseLoss); // VIOLATION (W4PM-EVD)
            }
        "#;
        let diags = check_source(code);
        let evd_violations: Vec<_> = diags.iter().filter(|d| d.code == "W4PM-EVD").collect();
        assert!(
            !evd_violations.is_empty(),
            "Should detect direct export on Raw evidence"
        );
    }

    #[test]
    fn test_direct_format_laundering() {
        let code = r#"
            fn ocel_to_xes(log: OcelLog) -> XesLog { // VIOLATION (W4PM-FMT)
                // Direct laundering without Project trait or LossReport
                let mut xes = XesLog::new();
                xes
            }
        "#;
        let diags = check_source(code);
        let fmt_violations: Vec<_> = diags.iter().filter(|d| d.code == "W4PM-FMT").collect();
        assert!(
            !fmt_violations.is_empty(),
            "Should detect direct ocel-to-xes format laundering function signature"
        );
    }

    #[test]
    fn test_lawful_projection_signature() {
        let code = r#"
            fn project_ocel_to_xes(log: OcelLog) -> Result<LossReport<OcelShape, XesShape, Vec<Dropped>>, Error> {
                // Returns LossReport, so it is loss-honest and not laundered.
                Ok(LossReport::new(ProjectionName("ocel-flatten"), LossPolicy::AllowLossWithReport, vec![]))
            }
        "#;
        let diags = check_source(code);
        let fmt_violations: Vec<_> = diags.iter().filter(|d| d.code == "W4PM-FMT").collect();
        assert!(
            fmt_violations.is_empty(),
            "Should not flag projection returning a LossReport"
        );
    }
}
