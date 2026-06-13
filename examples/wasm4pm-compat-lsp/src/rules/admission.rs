use proc_macro2::Span;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Expr, ExprMethodCall, File, FnArg, ImplItem, ItemImpl, Macro, Member, Pat, ReturnType, Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
}

/// The main entry point to check Rust source code content for admission & loss rule violations.
pub fn check_admission_rules(content: &str) -> Result<Vec<Diagnostic>, syn::Error> {
    let file = syn::parse_str::<File>(content)?;
    let mut diagnostics = Vec::new();
    let mut visitor = AdmissionVisitor {
        diagnostics: &mut diagnostics,
    };
    visitor.visit_file(&file);
    Ok(diagnostics)
}

struct AdmissionVisitor<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'ast> Visit<'ast> for AdmissionVisitor<'_> {
    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        if let Some((_, ref trait_path, _)) = i.trait_ {
            if let Some(last_seg) = trait_path.segments.last() {
                if last_seg.ident == "Admit" {
                    check_admit_impl(i, self.diagnostics);
                } else if last_seg.ident == "Project" {
                    check_project_impl(i, self.diagnostics);
                }
            }
        }
        visit::visit_item_impl(self, i);
    }
}

/// Checks an `impl Admit` block for explicit panic-inducing macros or unwraps.
fn check_admit_impl(item_impl: &ItemImpl, diagnostics: &mut Vec<Diagnostic>) {
    for item in &item_impl.items {
        if let ImplItem::Fn(impl_item_fn) = item {
            let mut fn_visitor = AdmitImplFnVisitor { diagnostics };
            fn_visitor.visit_impl_item_fn(impl_item_fn);
        }
    }
}

struct AdmitImplFnVisitor<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'ast> Visit<'ast> for AdmitImplFnVisitor<'_> {
    fn visit_macro(&mut self, i: &'ast Macro) {
        let macro_name = i.path.segments.last().map(|s| s.ident.to_string());
        if let Some(name) = macro_name {
            if matches!(
                name.as_str(),
                "panic"
                    | "todo"
                    | "unimplemented"
                    | "assert"
                    | "assert_eq"
                    | "assert_ne"
                    | "unreachable"
            ) {
                let span = i.path.segments.last().unwrap().ident.span();
                let start = span.start();
                self.diagnostics.push(Diagnostic {
                    code: "W4PM-ADM-001",
                    message: format!(
                        "Explicit panic-inducing macro `{}`! inside Admit implementation. Boundary decisions must return a Refusal instead.",
                        name
                    ),
                    line: start.line,
                    column: start.column,
                    severity: Severity::Error,
                });
            }
        }
        visit::visit_macro(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();
        if method_name == "unwrap" || method_name == "expect" {
            let span = i.method.span();
            let start = span.start();
            self.diagnostics.push(Diagnostic {
                code: "W4PM-ADM-001",
                message: format!(
                    "Forbidden method call `.{}` inside Admit implementation. Boundary decisions must return a Refusal instead.",
                    method_name
                ),
                line: start.line,
                column: start.column,
                severity: Severity::Error,
            });
        }
        visit::visit_expr_method_call(self, i);
    }
}

/// Checks an `impl Project` block to verify associated types, method signature, and correct LossReport construction.
fn check_project_impl(item_impl: &ItemImpl, diagnostics: &mut Vec<Diagnostic>) {
    let span = item_impl.span();
    let start = span.start();

    let mut has_from = false;
    let mut has_to = false;
    let mut has_lost = false;
    let mut has_reason = false;

    for item in &item_impl.items {
        if let ImplItem::Type(impl_item_type) = item {
            match impl_item_type.ident.to_string().as_str() {
                "From" => has_from = true,
                "To" => has_to = true,
                "Lost" => has_lost = true,
                "Reason" => has_reason = true,
                _ => {}
            }
        }
    }

    if !has_from || !has_to || !has_lost || !has_reason {
        diagnostics.push(Diagnostic {
            code: "W4PM-LOS-001",
            message: format!(
                "Project implementation is missing associated types. Found: From={}, To={}, Lost={}, Reason={}",
                has_from, has_to, has_lost, has_reason
            ),
            line: start.line,
            column: start.column,
            severity: Severity::Error,
        });
    }

    let mut project_fn = None;
    for item in &item_impl.items {
        if let ImplItem::Fn(impl_item_fn) = item {
            if impl_item_fn.sig.ident == "project" {
                project_fn = Some(impl_item_fn);
                break;
            }
        }
    }

    if let Some(impl_item_fn) = project_fn {
        let mut policy_param_name = None;
        let mut has_loss_policy = false;
        for input in &impl_item_fn.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                if let Type::Path(type_path) = &*pat_type.ty {
                    if type_path
                        .path
                        .segments
                        .last()
                        .is_some_and(|s| s.ident == "LossPolicy")
                    {
                        has_loss_policy = true;
                        if let Pat::Ident(pat_ident) = &*pat_type.pat {
                            policy_param_name = Some(pat_ident.ident.to_string());
                        }
                    }
                }
            }
        }

        let mut returns_loss_report_result = false;
        if let ReturnType::Type(_, ty) = &impl_item_fn.sig.output {
            if let Type::Path(type_path) = &**ty {
                if let Some(last_seg) = type_path.path.segments.last() {
                    if last_seg.ident == "Result" {
                        if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                            if let Some(syn::GenericArgument::Type(Type::Path(ok_path))) =
                                args.args.first()
                            {
                                if ok_path
                                    .path
                                    .segments
                                    .last()
                                    .is_some_and(|s| s.ident == "LossReport")
                                {
                                    returns_loss_report_result = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if !has_loss_policy || !returns_loss_report_result {
            diagnostics.push(Diagnostic {
                code: "W4PM-LOS-001",
                message: format!(
                    "project method signature does not conform to the Loss Law. Takes LossPolicy = {}, Returns Result<LossReport<...>> = {}",
                    has_loss_policy, returns_loss_report_result
                ),
                line: impl_item_fn.sig.ident.span().start().line,
                column: impl_item_fn.sig.ident.span().start().column,
                severity: Severity::Error,
            });
        }

        let mut loss_report_visitor = LossReportVisitor {
            diagnostics,
            policy_param_name,
        };
        loss_report_visitor.visit_impl_item_fn(impl_item_fn);
    } else {
        diagnostics.push(Diagnostic {
            code: "W4PM-LOS-001",
            message: "Project implementation is missing the `project` method.".to_string(),
            line: start.line,
            column: start.column,
            severity: Severity::Error,
        });
    }
}

struct LossReportVisitor<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
    policy_param_name: Option<String>,
}

impl<'ast> Visit<'ast> for LossReportVisitor<'_> {
    fn visit_expr(&mut self, i: &'ast Expr) {
        match i {
            Expr::Struct(expr_struct)
                if expr_struct
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "LossReport") =>
            {
                let mut found_projection = false;
                let mut found_policy = false;
                for field in &expr_struct.fields {
                    if let Member::Named(ident) = &field.member {
                        if ident == "projection" {
                            found_projection = true;
                            check_projection_expr(&field.expr, field.span(), self.diagnostics);
                        } else if ident == "policy" {
                            found_policy = true;
                            check_policy_expr(
                                &field.expr,
                                &self.policy_param_name,
                                field.span(),
                                self.diagnostics,
                            );
                        }
                    }
                }
                if !found_projection {
                    let start = expr_struct.span().start();
                    self.diagnostics.push(Diagnostic {
                        code: "W4PM-LOS-002",
                        message: "LossReport struct literal is missing 'projection' field."
                            .to_string(),
                        line: start.line,
                        column: start.column,
                        severity: Severity::Error,
                    });
                }
                if !found_policy {
                    let start = expr_struct.span().start();
                    self.diagnostics.push(Diagnostic {
                        code: "W4PM-LOS-002",
                        message: "LossReport struct literal is missing 'policy' field.".to_string(),
                        line: start.line,
                        column: start.column,
                        severity: Severity::Error,
                    });
                }
            }
            Expr::Call(expr_call) => {
                let is_loss_report_new = if let Expr::Path(expr_path) = &*expr_call.func {
                    let segments = &expr_path.path.segments;
                    if segments.len() >= 2 {
                        let last = &segments[segments.len() - 1].ident;
                        let prev = &segments[segments.len() - 2].ident;
                        prev == "LossReport" && last == "new"
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_loss_report_new {
                    if expr_call.args.len() >= 2 {
                        check_projection_expr(
                            &expr_call.args[0],
                            expr_call.args[0].span(),
                            self.diagnostics,
                        );
                        check_policy_expr(
                            &expr_call.args[1],
                            &self.policy_param_name,
                            expr_call.args[1].span(),
                            self.diagnostics,
                        );
                    } else {
                        let start = expr_call.span().start();
                        self.diagnostics.push(Diagnostic {
                            code: "W4PM-LOS-002",
                            message: "LossReport::new called with too few arguments (expected at least projection and policy).".to_string(),
                            line: start.line,
                            column: start.column,
                            severity: Severity::Error,
                        });
                    }
                }
            }
            _ => {}
        }
        visit::visit_expr(self, i);
    }
}

fn check_projection_expr(expr: &Expr, span: Span, diagnostics: &mut Vec<Diagnostic>) {
    let mut is_projection_name = false;
    let mut is_empty_or_trivial = false;

    match expr {
        Expr::Call(call) => {
            if let Expr::Path(expr_path) = &*call.func {
                if expr_path
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "ProjectionName")
                {
                    is_projection_name = true;
                    if let Some(Expr::Lit(expr_lit)) = call.args.first() {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            let val = lit_str.value();
                            if val.is_empty() || val == "temp" || val == "default" {
                                is_empty_or_trivial = true;
                            }
                        }
                    }
                }
            }
        }
        Expr::Path(_expr_path) => {
            // Referencing a variable or constant
            is_projection_name = true;
        }
        _ => {}
    }

    if !is_projection_name {
        let start = span.start();
        diagnostics.push(Diagnostic {
            code: "W4PM-LOS-002",
            message:
                "LossReport projection field should be constructed with a ProjectionName wrapper."
                    .to_string(),
            line: start.line,
            column: start.column,
            severity: Severity::Error,
        });
    } else if is_empty_or_trivial {
        let start = span.start();
        diagnostics.push(Diagnostic {
            code: "W4PM-LOS-002",
            message: "ProjectionName is empty or contains a trivial placeholder like 'temp' or 'default'.".to_string(),
            line: start.line,
            column: start.column,
            severity: Severity::Error,
        });
    }
}

fn check_policy_expr(
    expr: &Expr,
    policy_param_name: &Option<String>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut is_policy_param_ref = false;
    if let Some(param_name) = policy_param_name {
        if let Expr::Path(expr_path) = expr {
            if expr_path.path.is_ident(param_name) {
                is_policy_param_ref = true;
            }
        }
    } else {
        if let Expr::Path(expr_path) = expr {
            if expr_path.path.is_ident("policy") {
                is_policy_param_ref = true;
            }
        }
    }

    if !is_policy_param_ref {
        let start = span.start();
        diagnostics.push(Diagnostic {
            code: "W4PM-LOS-002",
            message: "LossReport policy field should carry forward the caller-provided LossPolicy parameter to prevent policy bypass.".to_string(),
            line: start.line,
            column: start.column,
            severity: Severity::Error,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_admit_impl() {
        let code = r#"
            impl Admit for MyType {
                type Raw = u8;
                type Admitted = u16;
                type Reason = &'static str;
                type Witness = Ocel20;

                fn admit(raw: Evidence<Self::Raw, Raw, Self::Witness>) -> Result<Admission<Self::Admitted, Self::Witness>, Refusal<Self::Reason, Self::Witness>> {
                    if raw.value > 0 {
                        Ok(Admission::new(raw.value as u16))
                    } else {
                        Err(Refusal::new("ZeroNotAllowed"))
                    }
                }
            }
        "#;
        let diagnostics = check_admission_rules(code).unwrap();
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_admit_impl_with_panic_and_unwrap() {
        let code = r#"
            impl Admit for MyType {
                type Raw = u8;
                type Admitted = u16;
                type Reason = &'static str;
                type Witness = Ocel20;

                fn admit(raw: Evidence<Self::Raw, Raw, Self::Witness>) -> Result<Admission<Self::Admitted, Self::Witness>, Refusal<Self::Reason, Self::Witness>> {
                    if raw.value == 0 {
                        panic!("zero values!");
                    }
                    let val = Some(raw.value).unwrap();
                    Ok(Admission::new(val as u16))
                }
            }
        "#;
        let diagnostics = check_admission_rules(code).unwrap();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].code, "W4PM-ADM-001");
        assert!(diagnostics[0].message.contains("panic"));
        assert_eq!(diagnostics[1].code, "W4PM-ADM-001");
        assert!(diagnostics[1].message.contains("unwrap"));
    }

    #[test]
    fn test_valid_project_impl() {
        let code = r#"
            impl Project for MyProjection {
                type From = Ocel;
                type To = Xes;
                type Lost = Vec<String>;
                type Reason = &'static str;

                fn project(self, policy: LossPolicy) -> Result<LossReport<Self::From, Self::To, Self::Lost>, Self::Reason> {
                    let items = vec![];
                    Ok(LossReport::new(ProjectionName("ocel-to-xes"), policy, items))
                }
            }
        "#;
        let diagnostics = check_admission_rules(code).unwrap();
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_invalid_project_impl_missing_types_and_bad_sig() {
        let code = r#"
            impl Project for MyProjection {
                type From = Ocel;
                // Missing To, Lost, Reason

                fn project(self) -> Result<(), &'static str> {
                    Ok(())
                }
            }
        "#;
        let diagnostics = check_admission_rules(code).unwrap();
        // W4PM-LOS-001 is triggered for missing associated types AND signature mismatch
        assert!(diagnostics.iter().any(|d| d.code == "W4PM-LOS-001"));
    }

    #[test]
    fn test_project_impl_with_bad_report() {
        let code = r#"
            impl Project for MyProjection {
                type From = Ocel;
                type To = Xes;
                type Lost = Vec<String>;
                type Reason = &'static str;

                fn project(self, policy: LossPolicy) -> Result<LossReport<Self::From, Self::To, Self::Lost>, Self::Reason> {
                    let items = vec![];
                    // Violation: Empty projection name and hardcoded policy (bypass)
                    Ok(LossReport::new(ProjectionName(""), LossPolicy::AllowLossWithReport, items))
                }
            }
        "#;
        let diagnostics = check_admission_rules(code).unwrap();
        // Should have W4PM-LOS-002 diagnostics for the ProjectionName and LossPolicy bypass
        let codes: Vec<&str> = diagnostics.iter().map(|d| d.code).collect();
        assert!(codes.contains(&"W4PM-LOS-002"));
        assert_eq!(diagnostics.len(), 2);
    }
}
