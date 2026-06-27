//! Hover and diagnostics view population via SPARQL.

use super::helpers::{run_select, term_to_string, term_to_u32};
use super::types::MaterializedViewStore;
use lsp_types_max::{Diagnostic, Hover, Position, Range};
use oxigraph::store::Store;
use url::Url;

const QUERY_HOVER: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:lsp-max:core:>
    SELECT ?srcDocUri ?srcStartLine ?srcStartChar ?srcEndLine ?srcEndChar ?hoverData WHERE {
      GRAPH ?g {
        {
          ?srcRange lsif:next ?resultSet .
          ?resultSet lsif:textDocument_hover ?hoverResult .
        } UNION {
          ?srcRange lsif:textDocument_hover ?hoverResult .
        }
        ?srcDoc lsif:contains ?srcRange .
        ?srcDoc max:uri ?srcDocUri .
        ?srcRange max:startLine ?srcStartLine ;
                  max:startCharacter ?srcStartChar ;
                  max:endLine ?srcEndLine ;
                  max:endCharacter ?srcEndChar .
        ?hoverResult max:hoverData ?hoverData .
      }
    }
";

const QUERY_LSIF_DIAGNOSTICS: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:lsp-max:core:>
    SELECT ?docUri ?diagData WHERE {
      GRAPH ?g {
        ?doc lsif:textDocument_diagnostic ?diagResult .
        ?diagResult max:diagnosticData ?diagData .
        ?doc max:uri ?docUri .
      }
    }
";

const QUERY_LIVE_DIAGNOSTICS: &str = "
    PREFIX max: <urn:lsp-max:core:>
    SELECT ?docUri ?message ?lawId WHERE {
      GRAPH ?g {
        ?diag a max:Diagnostic .
        ?diag max:message ?message .
        ?diag max:lawId ?lawId .
        ?diag max:docUri ?docUri .
      }
    }
";

pub(super) fn populate_hovers(store: &Store, views: &MaterializedViewStore) {
    use lsp_max_lsif::lsif::{HoverContents, HoverResultData};
    let Ok(sols) = run_select(store, QUERY_HOVER) else {
        return;
    };
    for sol in sols {
        let src_doc_uri = sol.get("srcDocUri").map(term_to_string).unwrap_or_default();
        let hover_data_str = sol.get("hoverData").map(term_to_string).unwrap_or_default();
        let Ok(src_url) = Url::parse(&src_doc_uri) else {
            continue;
        };
        let Ok(result_data) = serde_json::from_str::<HoverResultData>(&hover_data_str) else {
            continue;
        };
        let src_range = Range::new(
            Position::new(
                sol.get("srcStartLine").map(term_to_u32).unwrap_or(0),
                sol.get("srcStartChar").map(term_to_u32).unwrap_or(0),
            ),
            Position::new(
                sol.get("srcEndLine").map(term_to_u32).unwrap_or(0),
                sol.get("srcEndChar").map(term_to_u32).unwrap_or(0),
            ),
        );
        let hover_contents = match &result_data.contents {
            HoverContents::Markup(m) => lsp_types_max::HoverContents::Markup(m.clone()),
            HoverContents::String(s) => {
                lsp_types_max::HoverContents::Scalar(lsp_types_max::MarkedString::String(s.clone()))
            }
            HoverContents::MarkedString(ms) => lsp_types_max::HoverContents::Scalar(ms.clone()),
            HoverContents::MarkedStringArray(arr) => {
                lsp_types_max::HoverContents::Array(arr.clone())
            }
        };
        let lsp_hover = Hover {
            contents: hover_contents,
            range: result_data.range,
        };
        views
            .hovers
            .entry(src_url)
            .or_default()
            .push((src_range, lsp_hover));
    }
}

pub(super) fn populate_diagnostics(store: &Store, views: &MaterializedViewStore) {
    // LSIF-sourced diagnostics
    if let Ok(sols) = run_select(store, QUERY_LSIF_DIAGNOSTICS) {
        for sol in sols {
            let doc_uri = sol.get("docUri").map(term_to_string).unwrap_or_default();
            let diag_data_str = sol.get("diagData").map(term_to_string).unwrap_or_default();
            if let Ok(doc_url) = Url::parse(&doc_uri) {
                if let Ok(list) = serde_json::from_str::<Vec<Diagnostic>>(&diag_data_str) {
                    views.diagnostics.entry(doc_url).or_default().extend(list);
                }
            }
        }
    }

    // Live-LSP control-plane diagnostics
    if let Ok(sols) = run_select(store, QUERY_LIVE_DIAGNOSTICS) {
        for sol in sols {
            let doc_uri = sol.get("docUri").map(term_to_string).unwrap_or_default();
            let message = sol.get("message").map(term_to_string).unwrap_or_default();
            let law_id = sol.get("lawId").map(term_to_string).unwrap_or_default();
            if let Ok(doc_url) = Url::parse(&doc_uri) {
                let lsp_diag = Diagnostic {
                    range: Range::default(),
                    severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
                    code: Some(lsp_types_max::NumberOrString::String(law_id)),
                    source: Some("AdmittedGraphControlPlane".to_string()),
                    message,
                    ..Default::default()
                };
                views.diagnostics.entry(doc_url).or_default().push(lsp_diag);
            }
        }
    }
}
