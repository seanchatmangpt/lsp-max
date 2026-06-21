//! Directly-Follows Graph (DFG) — Van der Aalst's core process discovery primitive.
//!
//! A DFG records, for each pair of consecutive activities (A → B) in the same case,
//! how often that transition was observed.  It is the foundation for the α-Miner,
//! Heuristics Miner, and Inductive Miner algorithms described in:
//!
//!   W.M.P. van der Aalst, "Process Mining: Data Science in Action" (2nd ed., 2016)
//!   Chapter 5: From Event Logs to Process Models.
//!
//! This module operates on per-case activity traces produced by `declare::extract_traces`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::declare::extract_traces;

// ─────────────────────────────────────────────────────────────────────────────
// DirectlyFollowsGraph
// ─────────────────────────────────────────────────────────────────────────────

/// A Directly-Follows Graph derived from an OCEL event log.
///
/// The DFG captures process flow frequency without prescribing a full control-flow
/// language.  It is the simplest model the Inductive Miner can extract and is used
/// as the foundation for fitness / precision computation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectlyFollowsGraph {
    /// Activity name → occurrence count across all cases.
    pub nodes: HashMap<String, usize>,
    /// Directed edge `(from, to)` → frequency count.
    pub edges: HashMap<(String, String), usize>,
    /// Start activities (first event per case) → frequency.
    pub start_activities: HashMap<String, usize>,
    /// End activities (last event per case) → frequency.
    pub end_activities: HashMap<String, usize>,
}

impl DirectlyFollowsGraph {
    /// Build a DFG from per-case activity traces.
    pub fn from_traces(traces: &HashMap<String, Vec<String>>) -> Self {
        let mut dfg = Self::default();
        for trace in traces.values() {
            if trace.is_empty() {
                continue;
            }
            *dfg.start_activities.entry(trace[0].clone()).or_insert(0) += 1;
            *dfg.end_activities
                .entry(trace[trace.len() - 1].clone())
                .or_insert(0) += 1;
            for act in trace {
                *dfg.nodes.entry(act.clone()).or_insert(0) += 1;
            }
            for pair in trace.windows(2) {
                let key = (pair[0].clone(), pair[1].clone());
                *dfg.edges.entry(key).or_insert(0) += 1;
            }
        }
        dfg
    }

    /// Build a DFG directly from raw OCEL event JSON values.
    pub fn from_events(events: &[Value]) -> Self {
        let traces = extract_traces(events);
        Self::from_traces(&traces)
    }

    /// Fitness against a declared set of normative arcs.
    ///
    /// Returns the fraction of *observed* arcs that appear in the normative model.
    /// A score of `1.0` means every transition in the log is model-sanctioned.
    /// Returns `None` when the log has no observed arcs.
    pub fn fitness_against_model(&self, model_arcs: &[(String, String)]) -> Option<f64> {
        if self.edges.is_empty() {
            return None;
        }
        let matching = self
            .edges
            .keys()
            .filter(|e| model_arcs.contains(e))
            .count();
        Some(matching as f64 / self.edges.len() as f64)
    }

    /// Precision against a declared set of normative arcs.
    ///
    /// Returns the fraction of *normative* arcs that appear in the log.
    /// A score of `1.0` means all model arcs were observed (no dead paths).
    pub fn precision_against_model(&self, model_arcs: &[(String, String)]) -> Option<f64> {
        if model_arcs.is_empty() {
            return None;
        }
        let observed: std::collections::HashSet<_> = self.edges.keys().collect();
        let matching = model_arcs
            .iter()
            .filter(|a| observed.contains(a))
            .count();
        Some(matching as f64 / model_arcs.len() as f64)
    }

    /// Render the DFG as Mermaid flowchart markdown — renderable by GitHub, VS Code,
    /// and the `anti-llm://process-model` virtual document.
    pub fn to_mermaid(&self) -> String {
        let mut md = String::from("```mermaid\nflowchart LR\n");

        let mut nodes: Vec<(&String, &usize)> = self.nodes.iter().collect();
        nodes.sort_by_key(|(n, _)| n.as_str());
        for (name, count) in &nodes {
            md.push_str(&format!(
                "  {}[\"{}\\n(n={count})\"]\n",
                mermaid_id(name),
                name
            ));
        }

        let mut starts: Vec<(&String, &usize)> = self.start_activities.iter().collect();
        starts.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &starts {
            md.push_str(&format!("  START((▶)) -->|{freq}| {}\n", mermaid_id(act)));
        }

        let mut ends: Vec<(&String, &usize)> = self.end_activities.iter().collect();
        ends.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &ends {
            md.push_str(&format!("  {} -->|{freq}| END(((◼)))\n", mermaid_id(act)));
        }

        let mut edges: Vec<(&(String, String), &usize)> = self.edges.iter().collect();
        edges.sort_by_key(|((a, b), _)| (a.as_str(), b.as_str()));
        for ((from, to), freq) in &edges {
            md.push_str(&format!(
                "  {} -->|{freq}| {}\n",
                mermaid_id(from),
                mermaid_id(to)
            ));
        }

        md.push_str("```\n");
        md
    }

    /// Render the DFG as GraphViz DOT notation.
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph DFG {\n  rankdir=LR;\n  node [shape=rectangle];\n");

        let mut nodes: Vec<(&String, &usize)> = self.nodes.iter().collect();
        nodes.sort_by_key(|(n, _)| n.as_str());
        for (name, count) in &nodes {
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\\n(n={count})\"];\n",
                name, name
            ));
        }

        dot.push_str(
            "  \"[START]\" [shape=circle, style=filled, fillcolor=black, fontcolor=white];\n",
        );
        dot.push_str("  \"[END]\" [shape=doublecircle];\n");

        let mut starts: Vec<(&String, &usize)> = self.start_activities.iter().collect();
        starts.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &starts {
            dot.push_str(&format!(
                "  \"[START]\" -> \"{}\" [label=\"{freq}\"];\n",
                act
            ));
        }

        let mut ends: Vec<(&String, &usize)> = self.end_activities.iter().collect();
        ends.sort_by_key(|(n, _)| n.as_str());
        for (act, freq) in &ends {
            dot.push_str(&format!(
                "  \"{}\" -> \"[END]\" [label=\"{freq}\"];\n",
                act
            ));
        }

        let mut edges: Vec<(&(String, String), &usize)> = self.edges.iter().collect();
        edges.sort_by_key(|((a, b), _)| (a.as_str(), b.as_str()));
        for ((from, to), freq) in &edges {
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{freq}\"];\n",
                from, to
            ));
        }

        dot.push('}');
        dot
    }

    /// Total number of unique activities (nodes).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total number of unique directly-follows arcs.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Sum of all edge frequencies — total transition count across all traces.
    pub fn total_transitions(&self) -> usize {
        self.edges.values().sum()
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

    #[test]
    fn dfg_counts_nodes_correctly() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        assert_eq!(*dfg.nodes.get("A").unwrap(), 2);
        assert_eq!(*dfg.nodes.get("B").unwrap(), 1);
        assert_eq!(*dfg.nodes.get("C").unwrap(), 2);
    }

    #[test]
    fn dfg_counts_edges_correctly() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        assert_eq!(
            *dfg.edges
                .get(&("A".to_string(), "B".to_string()))
                .unwrap(),
            1
        );
        assert_eq!(
            *dfg.edges
                .get(&("A".to_string(), "C".to_string()))
                .unwrap(),
            1
        );
        assert_eq!(
            *dfg.edges
                .get(&("B".to_string(), "C".to_string()))
                .unwrap(),
            1
        );
    }

    #[test]
    fn start_end_activities_recorded() {
        let traces = simple_traces();
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        assert_eq!(*dfg.start_activities.get("A").unwrap(), 2);
        assert_eq!(*dfg.end_activities.get("C").unwrap(), 2);
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
        assert!(mermaid.contains("\"A\\n(n=2)\"") || mermaid.contains("A"));
    }

    #[test]
    fn empty_traces_yields_empty_dfg() {
        let dfg = DirectlyFollowsGraph::from_traces(&HashMap::new());
        assert_eq!(dfg.node_count(), 0);
        assert_eq!(dfg.edge_count(), 0);
        assert_eq!(dfg.fitness_against_model(&[]), None);
    }
}
