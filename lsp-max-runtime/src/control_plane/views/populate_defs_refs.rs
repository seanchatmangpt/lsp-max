//! Definitions and references view population via SPARQL.

use super::helpers::{run_select, term_to_string, term_to_u32};
use super::types::MaterializedViewStore;
use lsp_types_max::{Location, Position, Range, Uri};
use oxigraph::store::Store;
use url::Url;

const QUERY_DEFINITIONS: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:lsp-max:core:>
    SELECT ?srcDocUri ?srcStartLine ?srcStartChar ?srcEndLine ?srcEndChar ?destDocUri ?destStartLine ?destStartChar ?destEndLine ?destEndChar WHERE {
      GRAPH ?g {
        {
          ?srcRange lsif:next ?resultSet .
          ?resultSet lsif:textDocument_definition ?defResult .
          ?defResult lsif:item ?destRange .
        } UNION {
          ?srcRange lsif:textDocument_definition ?defResult .
          ?defResult lsif:item ?destRange .
        }
        ?srcDoc lsif:contains ?srcRange .
        ?srcDoc max:uri ?srcDocUri .
        ?destDoc lsif:contains ?destRange .
        ?destDoc max:uri ?destDocUri .
        ?srcRange max:startLine ?srcStartLine ;
                  max:startCharacter ?srcStartChar ;
                  max:endLine ?srcEndLine ;
                  max:endCharacter ?srcEndChar .
        ?destRange max:startLine ?destStartLine ;
                   max:startCharacter ?destStartChar ;
                   max:endLine ?destEndLine ;
                   max:endCharacter ?destEndChar .
      }
    }
";

const QUERY_REFERENCES: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:lsp-max:core:>
    SELECT ?srcDocUri ?srcStartLine ?srcStartChar ?srcEndLine ?srcEndChar ?refDocUri ?refStartLine ?refStartChar ?refEndLine ?refEndChar WHERE {
      GRAPH ?g {
        {
          ?srcRange lsif:next ?resultSet .
          ?resultSet lsif:textDocument_references ?refResult .
          ?refResult lsif:item ?refRange .
        } UNION {
          ?srcRange lsif:textDocument_references ?refResult .
          ?refResult lsif:item ?refRange .
        }
        ?srcDoc lsif:contains ?srcRange .
        ?srcDoc max:uri ?srcDocUri .
        ?refDoc lsif:contains ?refRange .
        ?refDoc max:uri ?refDocUri .
        ?srcRange max:startLine ?srcStartLine ;
                  max:startCharacter ?srcStartChar ;
                  max:endLine ?srcEndLine ;
                  max:endCharacter ?srcEndChar .
        ?refRange max:startLine ?refStartLine ;
                  max:startCharacter ?refStartChar ;
                  max:endLine ?refEndLine ;
                  max:endCharacter ?refEndChar .
      }
    }
";

pub(super) fn populate_definitions(store: &Store, views: &MaterializedViewStore) {
    let Ok(sols) = run_select(store, QUERY_DEFINITIONS) else {
        return;
    };
    for sol in sols {
        let src_doc_uri = sol.get("srcDocUri").map(term_to_string).unwrap_or_default();
        let dest_doc_uri = sol
            .get("destDocUri")
            .map(term_to_string)
            .unwrap_or_default();
        if let (Ok(src_url), Ok(dest_url)) = (Url::parse(&src_doc_uri), Url::parse(&dest_doc_uri)) {
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
            let dest_range = Range::new(
                Position::new(
                    sol.get("destStartLine").map(term_to_u32).unwrap_or(0),
                    sol.get("destStartChar").map(term_to_u32).unwrap_or(0),
                ),
                Position::new(
                    sol.get("destEndLine").map(term_to_u32).unwrap_or(0),
                    sol.get("destEndChar").map(term_to_u32).unwrap_or(0),
                ),
            );
            let dest_uri = dest_url.to_string().parse::<Uri>().unwrap();
            let loc = Location {
                uri: dest_uri,
                range: dest_range,
            };
            views
                .definitions
                .entry(src_url)
                .or_default()
                .push((src_range, loc));
        }
    }
}

pub(super) fn populate_references(store: &Store, views: &MaterializedViewStore) {
    let mut temp: std::collections::HashMap<(Url, Range), Vec<Location>> = Default::default();
    let Ok(sols) = run_select(store, QUERY_REFERENCES) else {
        return;
    };
    for sol in sols {
        let src_doc_uri = sol.get("srcDocUri").map(term_to_string).unwrap_or_default();
        let ref_doc_uri = sol.get("refDocUri").map(term_to_string).unwrap_or_default();
        if let (Ok(src_url), Ok(ref_url)) = (Url::parse(&src_doc_uri), Url::parse(&ref_doc_uri)) {
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
            let ref_range = Range::new(
                Position::new(
                    sol.get("refStartLine").map(term_to_u32).unwrap_or(0),
                    sol.get("refStartChar").map(term_to_u32).unwrap_or(0),
                ),
                Position::new(
                    sol.get("refEndLine").map(term_to_u32).unwrap_or(0),
                    sol.get("refEndChar").map(term_to_u32).unwrap_or(0),
                ),
            );
            let ref_uri = ref_url.to_string().parse::<Uri>().unwrap();
            let loc = Location {
                uri: ref_uri,
                range: ref_range,
            };
            temp.entry((src_url, src_range)).or_default().push(loc);
        }
    }
    for ((url, range), locs) in temp {
        views.references.entry(url).or_default().push((range, locs));
    }
}
