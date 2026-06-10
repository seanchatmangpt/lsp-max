// Structural validators for POWL models.
// Operate on wasm4pm_compat canonical types.
// Graph-theory checks (cycle-freedom, reachability) live here.
// Process intelligence (fitness, conformance, discovery) graduates to wasm4pm.

use lsp_types_max::{Diagnostic, DiagnosticSeverity, Position, Range};
use std::collections::{HashMap, HashSet};
use wasm4pm_compat::powl::{Powl, PowlNodeId, PowlNodeKind};

fn zero_range() -> Range {
    Range::new(Position::new(0, 0), Position::new(0, 0))
}

fn error_diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        range: zero_range(),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("powl-lsp".to_string()),
        message: message.into(),
        ..Diagnostic::default()
    }
}

/// Validate the structural invariants of a `Powl` model.
pub fn validate_powl(powl: &Powl) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    if powl.root.is_none() && !powl.nodes.is_empty() {
        diags.push(error_diag("POWL model has nodes but no root."));
    }

    // Walk every node and validate its kind-specific invariants.
    for node in &powl.nodes {
        match &node.kind {
            PowlNodeKind::Atom(_) | PowlNodeKind::Silent => {}
            PowlNodeKind::PartialOrder(children) => {
                diags.extend(validate_partial_order(children, powl));
            }
            PowlNodeKind::Choice(children) => {
                if children.is_empty() {
                    diags.push(error_diag(format!(
                        "Choice node {:?} has no branches.",
                        node.id
                    )));
                }
            }
            PowlNodeKind::Loop { body, redo: _ } => {
                if !node_exists(*body, powl) {
                    diags.push(error_diag(format!(
                        "Loop node {:?}: 'do' body {:?} does not exist.",
                        node.id, body
                    )));
                }
            }
            PowlNodeKind::ChoiceGraph { nodes, edges } => {
                diags.extend(validate_choice_graph(nodes, edges, powl));
            }
        }
    }

    diags
}

fn node_exists(id: PowlNodeId, powl: &Powl) -> bool {
    powl.nodes.iter().any(|n| n.id == id)
}

fn validate_partial_order(children: &[PowlNodeId], powl: &Powl) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let node_set: HashSet<usize> = children.iter().map(|id| id.0).collect();

    // Build adjacency from OrderEdges that touch these children.
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for id in &node_set {
        adj.insert(*id, vec![]);
    }
    for edge in &powl.edges {
        if node_set.contains(&edge.from.0) && node_set.contains(&edge.to.0) {
            adj.entry(edge.from.0).or_default().push(edge.to.0);
        }
    }

    if has_cycle_usize(&node_set, &adj) {
        diags.push(error_diag(
            "Partial order contains a cycle, which is not allowed.",
        ));
    }
    diags
}

fn validate_choice_graph(
    nodes: &[PowlNodeId],
    edges: &[wasm4pm_compat::powl::ChoiceGraphEdge],
    _powl: &Powl,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let node_set: HashSet<usize> = nodes.iter().map(|id| id.0).collect();

    if node_set.is_empty() {
        diags.push(error_diag("Choice graph must have at least one node."));
        return diags;
    }

    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut rev_adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for id in &node_set {
        adj.insert(*id, vec![]);
        rev_adj.insert(*id, vec![]);
    }
    for edge in edges {
        adj.entry(edge.from.0).or_default().push(edge.to.0);
        rev_adj.entry(edge.to.0).or_default().push(edge.from.0);
    }

    if has_cycle_usize(&node_set, &adj) {
        diags.push(error_diag(
            "Choice graph contains a cycle, which is not allowed.",
        ));
    }

    diags
}

fn get_reachable(start: usize, adj: &HashMap<usize, Vec<usize>>) -> HashSet<usize> {
    let mut visited = HashSet::new();
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if visited.insert(node) {
            if let Some(neighbors) = adj.get(&node) {
                for &n in neighbors {
                    if !visited.contains(&n) {
                        stack.push(n);
                    }
                }
            }
        }
    }
    visited
}

fn has_cycle_usize(node_ids: &HashSet<usize>, adj: &HashMap<usize, Vec<usize>>) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    for &id in node_ids {
        if !visited.contains(&id) && is_cyclic_util(id, adj, &mut visited, &mut rec_stack) {
            return true;
        }
    }
    false
}

fn is_cyclic_util(
    id: usize,
    adj: &HashMap<usize, Vec<usize>>,
    visited: &mut HashSet<usize>,
    rec_stack: &mut HashSet<usize>,
) -> bool {
    visited.insert(id);
    rec_stack.insert(id);

    if let Some(neighbors) = adj.get(&id) {
        for &n in neighbors {
            if !visited.contains(&n) {
                if is_cyclic_util(n, adj, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(&n) {
                return true;
            }
        }
    }
    rec_stack.remove(&id);
    false
}

// Suppress unused-import warning — get_reachable is available for future use.
#[allow(dead_code)]
fn _use_get_reachable() {
    let _ = get_reachable;
}
