use crate::powl_types::{ChoiceGraph, Loop, PartialOrder, PowlNode};
use lsp_types_max::{Diagnostic, DiagnosticSeverity, Position, Range};
use std::collections::{HashMap, HashSet};

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

/// Port of choiceGraph.ts `validateChoiceGraph`
pub fn validate_choice_graph(cg: &ChoiceGraph) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let node_ids: HashSet<String> = cg.nodes.iter().map(|n| n.id().to_string()).collect();

    // Rule 1: must have at least one node
    if node_ids.is_empty() {
        diags.push(error_diag("Choice graph must have at least one node."));
        return diags;
    }

    // Rule 2: valid start node
    let start_valid = node_ids.contains(&cg.start_node);
    if !start_valid {
        diags.push(error_diag(format!(
            "Choice graph startNode '{}' is not a valid node id.",
            cg.start_node
        )));
    }

    // Rule 3: valid end node
    let end_valid = node_ids.contains(&cg.end_node);
    if !end_valid {
        diags.push(error_diag(format!(
            "Choice graph endNode '{}' is not a valid node id.",
            cg.end_node
        )));
    }

    // Build adjacency list
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut rev_adj: HashMap<String, Vec<String>> = HashMap::new();
    for id in &node_ids {
        adj.insert(id.clone(), vec![]);
        rev_adj.insert(id.clone(), vec![]);
    }
    for [from, to] in &cg.edges {
        adj.entry(from.clone()).or_default().push(to.clone());
        rev_adj.entry(to.clone()).or_default().push(from.clone());
    }

    // Rule 4: reachability (only if both start/end valid)
    if start_valid && end_valid {
        let reachable_from_start = get_reachable(&cg.start_node, &adj);
        for id in &node_ids {
            if !reachable_from_start.contains(id) {
                diags.push(error_diag(format!(
                    "Node '{}' is not reachable from startNode '{}'.",
                    id, cg.start_node
                )));
            }
        }

        let can_reach_end = get_reachable(&cg.end_node, &rev_adj);
        for id in &node_ids {
            if !can_reach_end.contains(id) {
                diags.push(error_diag(format!(
                    "Node '{}' cannot reach endNode '{}'.",
                    id, cg.end_node
                )));
            }
        }
    }

    // Rule 5: must be acyclic
    if has_cycle(&node_ids, &adj) {
        diags.push(error_diag("Choice graph contains a cycle, which is not allowed."));
    }

    diags
}

fn get_reachable(start: &str, adj: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(node) = stack.pop() {
        if visited.insert(node.clone()) {
            if let Some(neighbors) = adj.get(&node) {
                for n in neighbors {
                    if !visited.contains(n) {
                        stack.push(n.clone());
                    }
                }
            }
        }
    }
    visited
}

fn has_cycle(node_ids: &HashSet<String>, adj: &HashMap<String, Vec<String>>) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    for id in node_ids {
        if !visited.contains(id) && is_cyclic_util(id, adj, &mut visited, &mut rec_stack) {
            return true;
        }
    }
    false
}

fn is_cyclic_util(
    id: &str,
    adj: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
) -> bool {
    visited.insert(id.to_string());
    rec_stack.insert(id.to_string());

    if let Some(neighbors) = adj.get(id) {
        for n in neighbors {
            if !visited.contains(n) {
                if is_cyclic_util(n, adj, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(n) {
                return true;
            }
        }
    }
    rec_stack.remove(id);
    false
}

/// Port of partialOrder.ts `validatePartialOrder`
pub fn validate_partial_order(po: &PartialOrder) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let mut node_ids: HashSet<String> = po.nodes.iter().map(|n| n.id().to_string()).collect();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    for [from, to] in &po.edges {
        node_ids.insert(from.clone());
        node_ids.insert(to.clone());
        adj.entry(from.clone()).or_default().push(to.clone());
    }
    for id in &node_ids {
        adj.entry(id.clone()).or_insert_with(Vec::new);
    }

    if has_cycle(&node_ids, &adj) {
        diags.push(error_diag("Partial order contains a cycle, which is not allowed."));
    }

    diags
}

/// Port of loop.ts `validateLoop`
pub fn validate_loop(lp: &Loop) -> Vec<Diagnostic> {
    // do_part and redo_part are non-Option Box<PowlNode>, so they always exist at Rust type
    // level. The TypeScript checks for null/undefined — here we just confirm the fields exist
    // (always true). No additional diagnostics needed unless downstream validation fails.
    let _ = (&lp.do_part, &lp.redo_part);
    Vec::new()
}

/// Recursive dispatcher: validates a node and all children
pub fn validate_node(node: &PowlNode) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    match node {
        PowlNode::Activity(_) => {}
        PowlNode::PartialOrder(po) => {
            diags.extend(validate_partial_order(po));
            for child in &po.nodes {
                diags.extend(validate_node(child));
            }
        }
        PowlNode::ChoiceGraph(cg) => {
            diags.extend(validate_choice_graph(cg));
            for child in &cg.nodes {
                diags.extend(validate_node(child));
            }
        }
        PowlNode::Loop(lp) => {
            diags.extend(validate_loop(lp));
            diags.extend(validate_node(&lp.do_part));
            diags.extend(validate_node(&lp.redo_part));
        }
    }
    diags
}
