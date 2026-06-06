//! Call hierarchy view population (prepare, incoming, outgoing) via SPARQL.

use super::helpers::{parse_symbol_kind, run_select, term_to_string, term_to_u32};
use super::lookups::contains_position;
use super::types::MaterializedViewStore;
use lsp_types_max::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, Position, Range, Uri,
};
use oxigraph::store::Store;
use std::collections::HashMap;
use url::Url;

const QUERY_CALL_PREPARE: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:tower-lsp-max:core:>
    SELECT ?srcDocUri ?srcStartLine ?srcStartChar ?srcEndLine ?srcEndChar
           ?itemDocUri ?itemName ?itemKind ?itemDetail
           ?itemStartLine ?itemStartChar ?itemEndLine ?itemEndChar
           ?itemFullStartLine ?itemFullStartChar ?itemFullEndLine ?itemFullEndChar WHERE {
      GRAPH ?g {
        { ?srcRange lsif:next ?resultSet . ?resultSet lsif:textDocument_callHierarchy ?callResult . }
        UNION { ?srcRange lsif:textDocument_callHierarchy ?callResult . }
        ?callResult lsif:item ?itemRange .
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

const QUERY_CALL_INCOMING: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:tower-lsp-max:core:>
    SELECT ?itemDocUri ?itemStartLine ?itemStartChar ?itemEndLine ?itemEndChar
           ?callerDocUri ?callerName ?callerKind ?callerDetail
           ?callerStartLine ?callerStartChar ?callerEndLine ?callerEndChar
           ?callerFullStartLine ?callerFullStartChar ?callerFullEndLine ?callerFullEndChar
           ?refStartLine ?refStartChar ?refEndLine ?refEndChar WHERE {
      GRAPH ?g {
        { ?refRange lsif:next ?resultSet . ?resultSet lsif:textDocument_references ?refResult . ?refResult lsif:item ?itemRange . }
        UNION { ?refRange lsif:textDocument_references ?refResult . ?refResult lsif:item ?itemRange . }
        ?itemDoc lsif:contains ?itemRange . ?itemDoc max:uri ?itemDocUri .
        ?itemRange max:startLine ?itemStartLine ; max:startCharacter ?itemStartChar ;
                   max:endLine ?itemEndLine ; max:endCharacter ?itemEndChar .
        ?callerDoc lsif:contains ?callerRange . ?callerDoc lsif:contains ?refRange . ?callerDoc max:uri ?callerDocUri .
        ?callerRange max:tagType \"definition\" .
        ?callerRange max:startLine ?callerStartLine ; max:startCharacter ?callerStartChar ;
                     max:endLine ?callerEndLine ; max:endCharacter ?callerEndChar .
        ?callerRange max:fullStartLine ?callerFullStartLine ; max:fullStartCharacter ?callerFullStartChar ;
                     max:fullEndLine ?callerFullEndLine ; max:fullEndCharacter ?callerFullEndChar .
        OPTIONAL { ?callerRange max:text ?callerName }
        OPTIONAL { ?callerRange max:symbolKind ?callerKind }
        OPTIONAL { ?callerRange max:detail ?callerDetail }
        ?refRange max:startLine ?refStartLine ; max:startCharacter ?refStartChar ;
                  max:endLine ?refEndLine ; max:endCharacter ?refEndChar .
      }
    }
";

const QUERY_CALL_OUTGOING: &str = "
    PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
    PREFIX max: <urn:tower-lsp-max:core:>
    SELECT ?itemDocUri ?itemStartLine ?itemStartChar ?itemEndLine ?itemEndChar
           ?calleeDocUri ?calleeName ?calleeKind ?calleeDetail
           ?calleeStartLine ?calleeStartChar ?calleeEndLine ?calleeEndChar
           ?calleeFullStartLine ?calleeFullStartChar ?calleeFullEndLine ?calleeFullEndChar
           ?refStartLine ?refStartChar ?refEndLine ?refEndChar WHERE {
      GRAPH ?g {
        { ?refRange lsif:next ?resultSet . ?resultSet lsif:textDocument_references ?refResult . ?refResult lsif:item ?calleeRange . }
        UNION { ?refRange lsif:textDocument_references ?refResult . ?refResult lsif:item ?calleeRange . }
        ?calleeDoc lsif:contains ?calleeRange . ?calleeDoc max:uri ?calleeDocUri .
        ?calleeRange max:startLine ?calleeStartLine ; max:startCharacter ?calleeStartChar ;
                     max:endLine ?calleeEndLine ; max:endCharacter ?calleeEndChar .
        OPTIONAL { ?calleeRange max:text ?calleeName }
        OPTIONAL { ?calleeRange max:symbolKind ?calleeKind }
        OPTIONAL { ?calleeRange max:detail ?calleeDetail }
        OPTIONAL { ?calleeRange max:fullStartLine ?calleeFullStartLine ; max:fullStartCharacter ?calleeFullStartChar ;
                                max:fullEndLine ?calleeFullEndLine ; max:fullEndCharacter ?calleeFullEndChar . }
        ?itemDoc lsif:contains ?itemRange . ?itemDoc lsif:contains ?refRange . ?itemDoc max:uri ?itemDocUri .
        ?itemRange max:tagType \"definition\" .
        ?itemRange max:startLine ?itemStartLine ; max:startCharacter ?itemStartChar ;
                   max:endLine ?itemEndLine ; max:endCharacter ?itemEndChar .
        ?itemRange max:fullStartLine ?itemFullStartLine ; max:fullStartCharacter ?itemFullStartChar ;
                   max:fullEndLine ?itemFullEndLine ; max:fullEndCharacter ?itemFullEndChar .
        ?refRange max:startLine ?refStartLine ; max:startCharacter ?refStartChar ;
                  max:endLine ?refEndLine ; max:endCharacter ?refEndChar .
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

fn mk_range(sl: u32, sc: u32, el: u32, ec: u32) -> Range {
    Range::new(Position::new(sl, sc), Position::new(el, ec))
}

fn src_range(sol: &Sol) -> Range {
    mk_range(
        u(sol, "srcStartLine"),
        u(sol, "srcStartChar"),
        u(sol, "srcEndLine"),
        u(sol, "srcEndChar"),
    )
}
fn item_range(sol: &Sol) -> Range {
    mk_range(
        u(sol, "itemStartLine"),
        u(sol, "itemStartChar"),
        u(sol, "itemEndLine"),
        u(sol, "itemEndChar"),
    )
}
fn item_full(sol: &Sol, fallback: Range) -> Range {
    mk_range(
        uf(sol, "itemFullStartLine", fallback.start.line),
        uf(sol, "itemFullStartChar", fallback.start.character),
        uf(sol, "itemFullEndLine", fallback.end.line),
        uf(sol, "itemFullEndChar", fallback.end.character),
    )
}
fn caller_range(sol: &Sol) -> Range {
    mk_range(
        u(sol, "callerStartLine"),
        u(sol, "callerStartChar"),
        u(sol, "callerEndLine"),
        u(sol, "callerEndChar"),
    )
}
fn caller_full(sol: &Sol, fallback: Range) -> Range {
    mk_range(
        uf(sol, "callerFullStartLine", fallback.start.line),
        uf(sol, "callerFullStartChar", fallback.start.character),
        uf(sol, "callerFullEndLine", fallback.end.line),
        uf(sol, "callerFullEndChar", fallback.end.character),
    )
}
fn callee_range(sol: &Sol) -> Range {
    mk_range(
        u(sol, "calleeStartLine"),
        u(sol, "calleeStartChar"),
        u(sol, "calleeEndLine"),
        u(sol, "calleeEndChar"),
    )
}
fn callee_full(sol: &Sol, fallback: Range) -> Range {
    mk_range(
        uf(sol, "calleeFullStartLine", fallback.start.line),
        uf(sol, "calleeFullStartChar", fallback.start.character),
        uf(sol, "calleeFullEndLine", fallback.end.line),
        uf(sol, "calleeFullEndChar", fallback.end.character),
    )
}
fn ref_range(sol: &Sol) -> Range {
    mk_range(
        u(sol, "refStartLine"),
        u(sol, "refStartChar"),
        u(sol, "refEndLine"),
        u(sol, "refEndChar"),
    )
}

pub(super) fn populate_call_hierarchy(store: &Store, views: &MaterializedViewStore) {
    // 5a. Prepare
    if let Ok(sols) = run_select(store, QUERY_CALL_PREPARE) {
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
                let item = CallHierarchyItem {
                    name: sol.get("itemName").map(term_to_string).unwrap_or_default(),
                    kind: parse_symbol_kind(
                        &sol.get("itemKind").map(term_to_string).unwrap_or_default(),
                    ),
                    tags: None,
                    detail: sol.get("itemDetail").map(term_to_string),
                    uri: item_url.to_string().parse().unwrap(),
                    range: ifr,
                    selection_range: ir,
                    data: None,
                };
                views
                    .call_hierarchy_prepare
                    .entry(src_url)
                    .or_default()
                    .push((sr, vec![item]));
            }
        }
    }

    // 5b. Incoming
    let mut temp_inc: HashMap<(Url, Range), Vec<(CallHierarchyItem, Range)>> = HashMap::new();
    if let Ok(sols) = run_select(store, QUERY_CALL_INCOMING) {
        for sol in sols {
            let item_doc = sol
                .get("itemDocUri")
                .map(term_to_string)
                .unwrap_or_default();
            let caller_doc = sol
                .get("callerDocUri")
                .map(term_to_string)
                .unwrap_or_default();
            if let (Ok(item_url), Ok(caller_url)) = (Url::parse(&item_doc), Url::parse(&caller_doc))
            {
                let ir = item_range(&sol);
                let cr = caller_range(&sol);
                let cfr = caller_full(&sol, cr);
                let rr = ref_range(&sol);
                if contains_position(&cfr, rr.start) {
                    let caller_item = CallHierarchyItem {
                        name: sol
                            .get("callerName")
                            .map(term_to_string)
                            .unwrap_or_default(),
                        kind: parse_symbol_kind(
                            &sol.get("callerKind")
                                .map(term_to_string)
                                .unwrap_or_default(),
                        ),
                        tags: None,
                        detail: sol.get("callerDetail").map(term_to_string),
                        uri: caller_url.to_string().parse().unwrap(),
                        range: cfr,
                        selection_range: cr,
                        data: None,
                    };
                    temp_inc
                        .entry((item_url, ir))
                        .or_default()
                        .push((caller_item, rr));
                }
            }
        }
    }
    for ((url, range), calls) in temp_inc {
        let mut grouped: HashMap<Uri, (CallHierarchyItem, Vec<Range>)> = HashMap::new();
        for (item, rr) in calls {
            grouped
                .entry(item.uri.clone())
                .or_insert((item, vec![]))
                .1
                .push(rr);
        }
        let list: Vec<CallHierarchyIncomingCall> = grouped
            .into_values()
            .map(|(item, ranges)| CallHierarchyIncomingCall {
                from: item,
                from_ranges: ranges,
            })
            .collect();
        views
            .call_hierarchy_incoming
            .entry(url)
            .or_default()
            .push((range, list));
    }

    // 5c. Outgoing
    let mut temp_out: HashMap<(Url, Range), Vec<(CallHierarchyItem, Range)>> = HashMap::new();
    if let Ok(sols) = run_select(store, QUERY_CALL_OUTGOING) {
        for sol in sols {
            let item_doc = sol
                .get("itemDocUri")
                .map(term_to_string)
                .unwrap_or_default();
            let callee_doc = sol
                .get("calleeDocUri")
                .map(term_to_string)
                .unwrap_or_default();
            if let (Ok(item_url), Ok(callee_url)) = (Url::parse(&item_doc), Url::parse(&callee_doc))
            {
                let ir = item_range(&sol);
                let ifr = item_full(&sol, ir);
                let cr = callee_range(&sol);
                let cfr = callee_full(&sol, cr);
                let rr = ref_range(&sol);
                if contains_position(&ifr, rr.start) {
                    let callee_item = CallHierarchyItem {
                        name: sol
                            .get("calleeName")
                            .map(term_to_string)
                            .unwrap_or_default(),
                        kind: parse_symbol_kind(
                            &sol.get("calleeKind")
                                .map(term_to_string)
                                .unwrap_or_default(),
                        ),
                        tags: None,
                        detail: sol.get("calleeDetail").map(term_to_string),
                        uri: callee_url.to_string().parse().unwrap(),
                        range: cfr,
                        selection_range: cr,
                        data: None,
                    };
                    temp_out
                        .entry((item_url, ir))
                        .or_default()
                        .push((callee_item, rr));
                }
            }
        }
    }
    for ((url, range), calls) in temp_out {
        let mut grouped: HashMap<Uri, (CallHierarchyItem, Vec<Range>)> = HashMap::new();
        for (item, rr) in calls {
            grouped
                .entry(item.uri.clone())
                .or_insert((item, vec![]))
                .1
                .push(rr);
        }
        let list: Vec<CallHierarchyOutgoingCall> = grouped
            .into_values()
            .map(|(item, ranges)| CallHierarchyOutgoingCall {
                to: item,
                from_ranges: ranges,
            })
            .collect();
        views
            .call_hierarchy_outgoing
            .entry(url)
            .or_default()
            .push((range, list));
    }
}
