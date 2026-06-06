//! Type hierarchy view population (prepare, supertypes, subtypes) via SPARQL.

use super::helpers::{parse_symbol_kind, run_select, term_to_string, term_to_u32};
use super::types::MaterializedViewStore;
use lsp_types_max::{Position, Range, TypeHierarchyItem};
use oxigraph::store::Store;
use url::Url;

const QUERY_TYPE_PREPARE: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:tower-lsp-max:core:>
    SELECT ?srcDocUri ?srcStartLine ?srcStartChar ?srcEndLine ?srcEndChar
           ?itemDocUri ?itemName ?itemKind ?itemDetail
           ?itemStartLine ?itemStartChar ?itemEndLine ?itemEndChar
           ?itemFullStartLine ?itemFullStartChar ?itemFullEndLine ?itemFullEndChar WHERE {
      GRAPH ?g {
        { ?srcRange lsif:next ?resultSet . ?resultSet lsif:textDocument_typeHierarchy ?typeResult . }
        UNION { ?srcRange lsif:textDocument_typeHierarchy ?typeResult . }
        ?typeResult lsif:item ?itemRange .
        ?srcDoc lsif:contains ?srcRange . ?srcDoc max:uri ?srcDocUri .
        ?srcRange max:startLine ?srcStartLine ; max:startCharacter ?srcStartChar ;
                  max:endLine ?srcEndLine ; max:endCharacter ?srcEndChar .
        ?itemDoc lsif:contains ?itemRange . ?itemDoc max:uri ?itemDocUri .
        ?itemRange max:startLine ?itemStartLine ; max:startCharacter ?itemStartChar ;
                   max:endLine ?itemEndLine ; max:endCharacter ?itemEndChar .
        OPTIONAL { ?itemRange max:text ?itemName }
        OPTIONAL { ?itemRange max:symbolKind ?itemKind }
        OPTIONAL { ?itemRange max:detail ?itemDetail }
        OPTIONAL { ?itemRange max:fullStartLine ?itemFullStartLine ; max:fullStartCharacter ?itemFullStartChar ;
                               max:fullEndLine ?itemFullEndLine ; max:fullEndCharacter ?itemFullEndChar . }
      }
    }
";

const QUERY_TYPE_SUPERTYPES: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:tower-lsp-max:core:>
    SELECT ?srcDocUri ?srcStartLine ?srcStartChar ?srcEndLine ?srcEndChar
           ?destDocUri ?destName ?destKind ?destDetail
           ?destStartLine ?destStartChar ?destEndLine ?destEndChar
           ?destFullStartLine ?destFullStartChar ?destFullEndLine ?destFullEndChar WHERE {
      GRAPH ?g {
        ?srcRange lsif:next ?resultSet .
        ?resultSet lsif:textDocument_typeHierarchy ?srcTypeResult .
        ?srcTypeResult lsif:next ?destTypeResult .
        ?destTypeResult lsif:item ?destRange .
        ?srcDoc lsif:contains ?srcRange . ?srcDoc max:uri ?srcDocUri .
        ?srcRange max:startLine ?srcStartLine ; max:startCharacter ?srcStartChar ;
                  max:endLine ?srcEndLine ; max:endCharacter ?srcEndChar .
        ?destDoc lsif:contains ?destRange . ?destDoc max:uri ?destDocUri .
        ?destRange max:startLine ?destStartLine ; max:startCharacter ?destStartChar ;
                   max:endLine ?destEndLine ; max:endCharacter ?destEndChar .
        OPTIONAL { ?destRange max:text ?destName }
        OPTIONAL { ?destRange max:symbolKind ?destKind }
        OPTIONAL { ?destRange max:detail ?destDetail }
        OPTIONAL { ?destRange max:fullStartLine ?destFullStartLine ; max:fullStartCharacter ?destFullStartChar ;
                               max:fullEndLine ?destFullEndLine ; max:fullEndCharacter ?destFullEndChar . }
      }
    }
";

const QUERY_TYPE_SUBTYPES: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:tower-lsp-max:core:>
    SELECT ?srcDocUri ?srcStartLine ?srcStartChar ?srcEndLine ?srcEndChar
           ?destDocUri ?destName ?destKind ?destDetail
           ?destStartLine ?destStartChar ?destEndLine ?destEndChar
           ?destFullStartLine ?destFullStartChar ?destFullEndLine ?destFullEndChar WHERE {
      GRAPH ?g {
        ?destRange lsif:next ?resultSet .
        ?resultSet lsif:textDocument_typeHierarchy ?destTypeResult .
        ?destTypeResult lsif:next ?srcTypeResult .
        ?srcTypeResult lsif:item ?srcRange .
        ?destDoc lsif:contains ?destRange . ?destDoc max:uri ?destDocUri .
        ?destRange max:startLine ?destStartLine ; max:startCharacter ?destStartChar ;
                   max:endLine ?destEndLine ; max:endCharacter ?destEndChar .
        ?srcDoc lsif:contains ?srcRange . ?srcDoc max:uri ?srcDocUri .
        ?srcRange max:startLine ?srcStartLine ; max:startCharacter ?srcStartChar ;
                  max:endLine ?srcEndLine ; max:endCharacter ?srcEndChar .
        OPTIONAL { ?srcRange max:text ?destName }
        OPTIONAL { ?srcRange max:symbolKind ?destKind }
        OPTIONAL { ?srcRange max:detail ?destDetail }
        OPTIONAL { ?srcRange max:fullStartLine ?destFullStartLine ; max:fullStartCharacter ?destFullStartChar ;
                              max:fullEndLine ?destFullEndLine ; max:fullEndCharacter ?destFullEndChar . }
      }
    }
";

type Sol = oxigraph::sparql::QuerySolution;

fn u(sol: &Sol, key: &str) -> u32 {
    sol.get(key).map(term_to_u32).unwrap_or(0)
}
fn uf(sol: &Sol, key: &str, fallback: u32) -> u32 {
    sol.get(key).map(term_to_u32).unwrap_or(fallback)
}

fn mk(sl: u32, sc: u32, el: u32, ec: u32) -> Range {
    Range::new(Position::new(sl, sc), Position::new(el, ec))
}

fn src_range(sol: &Sol) -> Range {
    mk(
        u(sol, "srcStartLine"),
        u(sol, "srcStartChar"),
        u(sol, "srcEndLine"),
        u(sol, "srcEndChar"),
    )
}
fn dest_range(sol: &Sol) -> Range {
    mk(
        u(sol, "destStartLine"),
        u(sol, "destStartChar"),
        u(sol, "destEndLine"),
        u(sol, "destEndChar"),
    )
}
fn item_range(sol: &Sol) -> Range {
    mk(
        u(sol, "itemStartLine"),
        u(sol, "itemStartChar"),
        u(sol, "itemEndLine"),
        u(sol, "itemEndChar"),
    )
}

fn dest_full(sol: &Sol, fallback: Range) -> Range {
    mk(
        uf(sol, "destFullStartLine", fallback.start.line),
        uf(sol, "destFullStartChar", fallback.start.character),
        uf(sol, "destFullEndLine", fallback.end.line),
        uf(sol, "destFullEndChar", fallback.end.character),
    )
}

fn item_full(sol: &Sol, fallback: Range) -> Range {
    mk(
        uf(sol, "itemFullStartLine", fallback.start.line),
        uf(sol, "itemFullStartChar", fallback.start.character),
        uf(sol, "itemFullEndLine", fallback.end.line),
        uf(sol, "itemFullEndChar", fallback.end.character),
    )
}

fn type_item_from(
    sol: &Sol,
    name_key: &str,
    kind_key: &str,
    detail_key: &str,
    url: Url,
    range: Range,
    full: Range,
) -> TypeHierarchyItem {
    TypeHierarchyItem {
        name: sol.get(name_key).map(term_to_string).unwrap_or_default(),
        kind: parse_symbol_kind(&sol.get(kind_key).map(term_to_string).unwrap_or_default()),
        tags: None,
        detail: sol.get(detail_key).map(term_to_string),
        uri: url.to_string().parse().unwrap(),
        range: full,
        selection_range: range,
        data: None,
    }
}

pub(super) fn populate_type_hierarchy(store: &Store, views: &MaterializedViewStore) {
    // 6a. Prepare
    if let Ok(sols) = run_select(store, QUERY_TYPE_PREPARE) {
        for sol in sols {
            let src_doc = sol.get("srcDocUri").map(term_to_string).unwrap_or_default();
            let item_doc = sol
                .get("itemDocUri")
                .map(term_to_string)
                .unwrap_or_default();
            if let (Ok(src_url), Ok(item_url)) = (Url::parse(&src_doc), Url::parse(&item_doc)) {
                let sr = src_range(&sol);
                let ir = item_range(&sol);
                let ifr = item_full(&sol, ir);
                let ti = type_item_from(
                    &sol,
                    "itemName",
                    "itemKind",
                    "itemDetail",
                    item_url,
                    ir,
                    ifr,
                );
                views
                    .type_hierarchy_prepare
                    .entry(src_url)
                    .or_default()
                    .push((sr, vec![ti]));
            }
        }
    }

    // 6b. Supertypes
    if let Ok(sols) = run_select(store, QUERY_TYPE_SUPERTYPES) {
        for sol in sols {
            let src_doc = sol.get("srcDocUri").map(term_to_string).unwrap_or_default();
            let dest_doc = sol
                .get("destDocUri")
                .map(term_to_string)
                .unwrap_or_default();
            if let (Ok(src_url), Ok(dest_url)) = (Url::parse(&src_doc), Url::parse(&dest_doc)) {
                let sr = src_range(&sol);
                let dr = dest_range(&sol);
                let dfr = dest_full(&sol, dr);
                let ti = type_item_from(
                    &sol,
                    "destName",
                    "destKind",
                    "destDetail",
                    dest_url,
                    dr,
                    dfr,
                );
                views
                    .type_hierarchy_supertypes
                    .entry(src_url)
                    .or_default()
                    .push((sr, vec![ti]));
            }
        }
    }

    // 6c. Subtypes
    if let Ok(sols) = run_select(store, QUERY_TYPE_SUBTYPES) {
        for sol in sols {
            let src_doc = sol.get("srcDocUri").map(term_to_string).unwrap_or_default();
            let dest_doc = sol
                .get("destDocUri")
                .map(term_to_string)
                .unwrap_or_default();
            if let (Ok(src_url), Ok(dest_url)) = (Url::parse(&src_doc), Url::parse(&dest_doc)) {
                let sr = src_range(&sol);
                let dr = dest_range(&sol);
                // subtypes: the "dest" full fields carry the subtype item's full span
                let dfr = dest_full(&sol, sr);
                let ti =
                    type_item_from(&sol, "destName", "destKind", "destDetail", src_url, sr, dfr);
                views
                    .type_hierarchy_subtypes
                    .entry(dest_url)
                    .or_default()
                    .push((dr, vec![ti]));
            }
        }
    }
}
