// Thin JSON → wasm4pm_compat::powl::Powl parser.
// Structural invariants (cycle-freedom, reachability) are enforced downstream
// by validators/mod.rs; discovery, replay, and conformance graduate to wasm4pm.

use serde_json::Value;
use wasm4pm_compat::powl::{ChoiceGraphEdge, OrderEdge, Powl, PowlNode, PowlNodeId, PowlNodeKind};

/// Parse a JSON value into a `Powl` arena.  Returns `Err` with a human-readable
/// message on structural problems (unknown `type`, missing required fields).
pub fn parse_powl(v: &Value) -> Result<Powl, String> {
    let mut powl = Powl::new();
    let root_id = parse_node(v, &mut powl)?;
    powl.root = Some(root_id);
    Ok(powl)
}

fn parse_node(v: &Value, powl: &mut Powl) -> Result<PowlNodeId, String> {
    let kind_str = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or("missing 'type' field")?;

    let id_str = v.get("id").and_then(Value::as_str).unwrap_or("?");
    let id = PowlNodeId(powl.nodes.len());

    let kind = match kind_str {
        "activity" => PowlNodeKind::Atom(id_str.to_string()),
        "silent" => PowlNodeKind::Silent,
        "partial_order" => {
            let children = parse_children(v, powl)?;
            let edges = parse_order_edges(v, powl)?;
            powl.edges.extend(edges);
            PowlNodeKind::PartialOrder(children)
        }
        "choice" => {
            let children = parse_children(v, powl)?;
            PowlNodeKind::Choice(children)
        }
        "loop" => {
            let body = v
                .get("do")
                .ok_or_else(|| "loop missing 'do' field".to_string())
                .and_then(|n| parse_node(n, powl))?;
            let redo = v.get("redo").map(|n| parse_node(n, powl)).transpose()?;
            PowlNodeKind::Loop { body, redo }
        }
        "choice_graph" => {
            let children = parse_children(v, powl)?;
            let cg_edges = parse_choice_graph_edges(v, powl)?;
            PowlNodeKind::ChoiceGraph {
                nodes: children,
                edges: cg_edges,
            }
        }
        other => return Err(format!("unknown POWL node type '{other}'")),
    };

    // Reserve the slot (id was computed before child pushes for Atom/Silent;
    // for compound nodes children are already pushed, id still correct because
    // we computed it before any push for this node).
    powl.nodes.push(PowlNode {
        id,
        kind,
        witness: core::marker::PhantomData,
    });
    Ok(id)
}

fn parse_children(v: &Value, powl: &mut Powl) -> Result<Vec<PowlNodeId>, String> {
    let arr = v
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("missing 'nodes' array")?;
    arr.iter().map(|n| parse_node(n, powl)).collect()
}

fn parse_order_edges(v: &Value, powl: &mut Powl) -> Result<Vec<OrderEdge>, String> {
    let Some(arr) = v.get("edges").and_then(Value::as_array) else {
        return Ok(vec![]);
    };
    arr.iter()
        .map(|e| {
            let pair = e.as_array().ok_or("edge must be [from, to] array")?;
            if pair.len() != 2 {
                return Err("edge array must have exactly 2 elements".to_string());
            }
            let from_str = pair[0].as_str().ok_or("edge 'from' must be a string")?;
            let to_str = pair[1].as_str().ok_or("edge 'to' must be a string")?;
            // Resolve ids by label search in already-pushed nodes.
            let from = resolve_id(from_str, powl)?;
            let to = resolve_id(to_str, powl)?;
            Ok(OrderEdge { from, to })
        })
        .collect()
}

fn parse_choice_graph_edges(v: &Value, powl: &Powl) -> Result<Vec<ChoiceGraphEdge>, String> {
    let Some(arr) = v.get("edges").and_then(Value::as_array) else {
        return Ok(vec![]);
    };
    arr.iter()
        .map(|e| {
            let pair = e.as_array().ok_or("CG edge must be [from, to]")?;
            if pair.len() != 2 {
                return Err("CG edge must have exactly 2 elements".to_string());
            }
            let from_str = pair[0].as_str().ok_or("CG edge 'from' must be a string")?;
            let to_str = pair[1].as_str().ok_or("CG edge 'to' must be a string")?;
            let from = resolve_id(from_str, powl)?;
            let to = resolve_id(to_str, powl)?;
            Ok(ChoiceGraphEdge { from, to })
        })
        .collect()
}

fn resolve_id(label: &str, powl: &Powl) -> Result<PowlNodeId, String> {
    powl.nodes
        .iter()
        .find(|n| matches!(&n.kind, PowlNodeKind::Atom(l) if l == label))
        .map(|n| n.id)
        .ok_or_else(|| format!("unresolved node id '{label}'"))
}
