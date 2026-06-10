use oxigraph::sparql::QueryResults;
use lsp_max_protocol::{LawAxis, MaxDiagnostic, Repairability, Terminality};

pub(super) fn term_to_string(term: &oxigraph::model::Term) -> String {
    match term {
        oxigraph::model::Term::NamedNode(n) => n.as_str().to_string(),
        oxigraph::model::Term::BlankNode(b) => b.as_str().to_string(),
        oxigraph::model::Term::Literal(l) => l.value().to_string(),
        oxigraph::model::Term::Triple(t) => format!("{:?}", t),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_diagnostic_detailed(
    id: &str,
    message: &str,
    invariant: &str,
    law_axis: LawAxis,
    observed: serde_json::Value,
    expected: serde_json::Value,
    repairability: Repairability,
    terminality: Terminality,
) -> MaxDiagnostic {
    MaxDiagnostic {
        diagnostic_id: id.to_string(),
        law_id: invariant.to_string(),
        law_axis,
        violated_axes: vec!["conformance".to_string()],
        violated_invariant: invariant.to_string(),
        observed_state: observed,
        expected_state: expected,
        repairability,
        terminality,
        lsp: lsp_types_max::Diagnostic {
            range: lsp_types_max::Range::default(),
            severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
            code: Some(lsp_types_max::NumberOrString::String(invariant.to_string())),
            source: Some("AdmittedGraphControlPlane".to_string()),
            message: message.to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn run_ask(store: &oxigraph::store::Store, query: &str) -> Result<bool, String> {
    let evaluator = oxigraph::sparql::SparqlEvaluator::new();
    let parsed = evaluator.parse_query(query).map_err(|e| e.to_string())?;
    match parsed
        .on_store(store)
        .execute()
        .map_err(|e| e.to_string())?
    {
        QueryResults::Boolean(val) => Ok(val),
        _ => Err("Expected boolean query result".to_string()),
    }
}

pub(super) fn run_select(
    store: &oxigraph::store::Store,
    query: &str,
) -> Result<Vec<oxigraph::sparql::QuerySolution>, String> {
    let evaluator = oxigraph::sparql::SparqlEvaluator::new();
    let parsed = evaluator.parse_query(query).map_err(|e| e.to_string())?;
    match parsed
        .on_store(store)
        .execute()
        .map_err(|e| e.to_string())?
    {
        QueryResults::Solutions(solutions) => {
            let mut results = Vec::new();
            for sol in solutions {
                results.push(sol.map_err(|e| e.to_string())?);
            }
            Ok(results)
        }
        _ => Err("Expected solutions query result".to_string()),
    }
}
