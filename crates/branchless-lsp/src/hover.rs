//! Hover provider (agent contract: `hover`).
//!
//! Composes the Tier 2 court report into a hover card: per-function branch
//! verdict, constant-time characterization, and residual classification
//! from `attributes::classify`.

use crate::ast_linter::LintReport;
use crate::attributes::{classify, Classification, ParsedAttrs};
use crate::disasm_court::DisasmReport;

/// Render a hover card for a function verdict.
pub fn hover_card(
    fn_name: &str,
    court: &DisasmReport,
    lint: &LintReport,
    attrs: &ParsedAttrs,
) -> Option<String> {
    let verdict = court.functions.iter().find(|f| f.name.contains(fn_name))?;
    let class = classify(attrs, verdict.branch_count > 0);
    let class_label = match class {
        Classification::Admitted => "ADMITTED",
        Classification::HintResidual => "HINT-RESIDUAL",
        _ => "ERROR-REFUSED",
    };
    let andon_count = lint.hits.len();
    Some(format!(
        "**branchless-lsp** `{fn_name}`\n\n\
         - Tier 2 verdict: **{class_label}** ({n} branch instructions)\n\
         - Tier 1 ANDON hits: {andon_count}\n\
         - Constant-time: {ct}",
        n = verdict.branch_count,
        ct = if verdict.is_branchless {
            "yes (no control flow observed)"
        } else {
            "NO (control flow observed)"
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FnVerdict;

    #[test]
    fn hover_card_renders_admitted() {
        let court = DisasmReport {
            functions: vec![FnVerdict {
                name: "probe::f".into(),
                branch_count: 0,
                alu_count: 0,
                load_store_count: 0,
                other_count: 0,
                is_branchless: true,
            }],
        };
        let card = hover_card("f", &court, &LintReport::default(), &ParsedAttrs::default())
            .expect("card for known fn");
        assert!(card.contains("ADMITTED"));
        assert!(card.contains("Constant-time: yes"));
    }

    #[test]
    fn hover_card_flips_to_refused_on_branches() {
        let court = DisasmReport {
            functions: vec![FnVerdict {
                name: "probe::g".into(),
                branch_count: 2,
                alu_count: 0,
                load_store_count: 0,
                other_count: 0,
                is_branchless: false,
            }],
        };
        let card = hover_card("g", &court, &LintReport::default(), &ParsedAttrs::default())
            .expect("card for known fn");
        assert!(card.contains("ERROR-REFUSED"));
    }

    #[test]
    fn hover_unknown_fn_returns_none() {
        let court = DisasmReport::default();
        assert!(hover_card(
            "nope",
            &court,
            &LintReport::default(),
            &ParsedAttrs::default()
        )
        .is_none());
    }
}
