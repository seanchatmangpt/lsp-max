//! Directly-Follows Graph — thin wrapper over wasm4pm's `DFG` type.
//!
//! DFG construction delegates to wasm4pm's data model (`wasm4pm::models::DFG`).
//! No custom DFG algorithm is implemented here; this module provides the
//! compositor-specific adapter surface: trace-based construction, fitness/precision
//! scoring against normative arcs, and Mermaid/DOT rendering.
//!
//! Theory: W.M.P. van der Aalst, "Process Mining: Data Science in Action" (2016) ch. 5.

use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

use crate::declare::extract_traces;
use wasm4pm::models::{DFGNode, DirectlyFollowsRelation, DFG};

// ─────────────────────────────────────────────────────────────────────────────
// DirectlyFollowsGraph
// ─────────────────────────────────────────────────────────────────────────────

/// A Directly-Follows Graph backed by wasm4pm's `DFG` data model.
///
/// The inner `DFG` owns all node, edge, and frequency data.  Construction from
/// per-case traces and from raw OCEL events is provided here as compositor-
/// specific adapters; the data model itself is wasm4pm's authoritative type.
#[derive(Debug, Clone)]
pub struct DirectlyFollowsGraph(pub DFG);

impl DirectlyFollowsGraph {
    /// Build a DFG from per-case activity traces.
    ///
    /// Counts are accumulated via HashMap then projected into the wasm4pm `DFG`
    /// Vec/BTreeMap types so the result is immediately usable by wasm4pm consumers.
    pub fn from_traces(traces: &HashMap<String, Vec<String>>) -> Self {
        let mut node_freq: HashMap<String, usize> = HashMap::new();
        let mut edge_freq: HashMap<(String, String), usize> = HashMap::new();
        let mut start_freq: BTreeMap<String, usize> = BTreeMap::new();
        let mut end_freq: BTreeMap<String, usize> = BTreeMap::new();

        for trace in traces.values() {
            if trace.is_empty() {
                continue;
            }
            *start_freq.entry(trace[0].clone()).or_insert(0) += 1;
            *end_freq.entry(trace[trace.len() - 1].clone()).or_insert(0) += 1;
            for act in trace {
                *node_freq.entry(act.clone()).or_insert(0) += 1;
            }
            for pair in trace.windows(2) {
                *edge_freq
                    .entry((pair[0].clone(), pair[1].clone()))
                    .or_insert(0) += 1;
            }
        }

        let mut nodes: Vec<DFGNode> = node_freq
            .into_iter()
            .map(|(id, frequency)| DFGNode {
                label: id.clone(),
                id,
                frequency,
            })
            .collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        let mut edges: Vec<DirectlyFollowsRelation> = edge_freq
            .into_iter()
            .map(|((from, to), frequency)| DirectlyFollowsRelation {
                from,
                to,
                frequency,
            })
            .collect();
        edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

        Self(DFG {
            nodes,
            edges,
            start_activities: start_freq,
            end_activities: end_freq,
        })
    }

    /// Build a DFG directly from raw OCEL event JSON values.
    pub fn from_events(events: &[Value]) -> Self {
        let traces = extract_traces(events);
        Self::from_traces(&traces)
    }

    /// Fitness: fraction of *observed* arcs that appear in the normative model.
    ///
    /// Returns `None` when the log has no observed arcs.
    pub fn fitness_against_model(&self, model_arcs: &[(String, String)]) -> Option<f64> {
        if self.0.edges.is_empty() {
            return None;
        }
        let matching = self
            .0
            .edges
            .iter()
            .filter(|e| model_arcs.iter().any(|(a, b)| a == &e.from && b == &e.to))
            .count();
        Some(matching as f64 / self.0.edges.len() as f64)
    }

    /// Precision: fraction of *normative* arcs that appear in the observed log.
    ///
    /// Returns `None` when the normative model has no arcs.
    pub fn precision_against_model(&self, model_arcs: &[(String, String)]) -> Option<f64> {
        if model_arcs.is_empty() {
            return None;
        }
        let matching = model_arcs
            .iter()
            .filter(|(a, b)| self.0.edges.iter().any(|e| e.from == *a && e.to == *b))
            .count();
        Some(matching as f64 / model_arcs.len() as f64)
    }

    /// Render the DFG as Mermaid flowchart markdown — renderable by GitHub, VS Code,
    /// and the `anti-llm://process-model` virtual document.
    pub fn to_mermaid(&self) -> String {
        let mut md = String::from("```mermaid\nflowchart LR\n");

        for node in &self.0.nodes {
            md.push_str(&format!(
                "  {}[\"{}\\n(n={})\"]\n",
                mermaid_id(&node.id),
                node.label,
                node.frequency
            ));
        }

        let mut starts: Vec<(&String, &usize)> = self.0.start_activities.iter().collect();
        starts.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &starts {
            md.push_str(&format!("  START((▶)) -->|{freq}| {}\n", mermaid_id(act)));
        }

        let mut ends: Vec<(&String, &usize)> = self.0.end_activities.iter().collect();
        ends.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &ends {
            md.push_str(&format!("  {} -->|{freq}| END(((◼)))\n", mermaid_id(act)));
        }

        for edge in &self.0.edges {
            md.push_str(&format!(
                "  {} -->|{}| {}\n",
                mermaid_id(&edge.from),
                edge.frequency,
                mermaid_id(&edge.to)
            ));
        }

        md.push_str("```\n");
        md
    }

    /// Render the DFG as GraphViz DOT notation.
    pub fn to_dot(&self) -> String {
        let mut dot =
            String::from("digraph DFG {\n  rankdir=LR;\n  node [shape=rectangle];\n");

        for node in &self.0.nodes {
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\\n(n={})\"];\n",
                node.id, node.label, node.frequency
            ));
        }

        dot.push_str(
            "  \"[START]\" [shape=circle, style=filled, fillcolor=black, fontcolor=white];\n",
        );
        dot.push_str("  \"[END]\" [shape=doublecircle];\n");

        let mut starts: Vec<(&String, &usize)> = self.0.start_activities.iter().collect();
        starts.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &starts {
            dot.push_str(&format!(
                "  \"[START]\" -> \"{}\" [label=\"{freq}\"];\n",
                act
            ));
        }

        let mut ends: Vec<(&String, &usize)> = self.0.end_activities.iter().collect();
        ends.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &ends {
            dot.push_str(&format!(
                "  \"{}\" -> \"[END]\" [label=\"{freq}\"];\n",
                act
            ));
        }

        for edge in &self.0.edges {
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                edge.from, edge.to, edge.frequency
            ));
        }

        dot.push('}');
        dot
    }

    /// Total number of unique activities (nodes).
    pub fn node_count(&self) -> usize {
        self.0.nodes.len()
    }

    /// Total number of unique directly-follows arcs.
    pub fn edge_count(&self) -> usize {
        self.0.edges.len()
    }

    /// Sum of all edge frequencies — total transition count across all traces.
    pub fn total_transitions(&self) -> usize {
        self.0.edges.iter().map(|e| e.frequency).sum()
    }
}

fn mermaid_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_traces() -> HashMap<String, Vec<String>> {
        let mut t = HashMap::new();
        t.insert(
            "case1".to_string(),
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
        );
        t.insert(
            "case2".to_string(),
            vec!["A".to_string(), "C".to_string()],
        );
        t
    }

    fn node_freq(dfg: &DirectlyFollowsGraph, name: &str) -> usize {
        dfg.0
            .nodes
            .iter()
            .find(|n| n.id == name)
            .map(|n| n.frequency)
            .unwrap_or(0)
    }

    fn edge_freq(dfg: &DirectlyFollowsGraph, from: &str, to: &str) -> usize {
        dfg.0
            .edges
            .iter()
            .find(|e| e.from == from && e.to == to)
            .map(|e| e.frequency)
            .unwrap_or(0)
    }

    #[test]
    fn dfg_counts_nodes_correctly() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        assert_eq!(node_freq(&dfg, "A"), 2);
        assert_eq!(node_freq(&dfg, "B"), 1);
        assert_eq!(node_freq(&dfg, "C"), 2);
    }

    #[test]
    fn dfg_counts_edges_correctly() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        assert_eq!(edge_freq(&dfg, "A", "B"), 1);
        assert_eq!(edge_freq(&dfg, "A", "C"), 1);
        assert_eq!(edge_freq(&dfg, "B", "C"), 1);
    }

    #[test]
    fn start_end_activities_recorded() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        assert_eq!(*dfg.0.start_activities.get("A").unwrap(), 2);
        assert_eq!(*dfg.0.end_activities.get("C").unwrap(), 2);
    }

    #[test]
    fn fitness_perfect_when_all_arcs_in_model() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        let model_arcs = vec![
            ("A".to_string(), "B".to_string()),
            ("A".to_string(), "C".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        assert_eq!(dfg.fitness_against_model(&model_arcs), Some(1.0));
    }

    #[test]
    fn fitness_zero_when_no_arcs_in_model() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        let model_arcs = vec![("X".to_string(), "Y".to_string())];
        assert_eq!(dfg.fitness_against_model(&model_arcs), Some(0.0));
    }

    #[test]
    fn mermaid_output_contains_node_names() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        let mermaid = dfg.to_mermaid();
        assert!(mermaid.contains("flowchart LR"));
        assert!(mermaid.contains('A'));
    }

    #[test]
    fn empty_traces_yields_empty_dfg() {
        let dfg = DirectlyFollowsGraph::from_traces(&HashMap::new());
        assert_eq!(dfg.node_count(), 0);
        assert_eq!(dfg.edge_count(), 0);
        assert_eq!(dfg.fitness_against_model(&[]), None);
    }
}
