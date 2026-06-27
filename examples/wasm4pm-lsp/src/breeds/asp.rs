//! `asp` — Answer Set Programming breed (Gelfond & Lifschitz 1991).
//!
//! Family: FormalMethods
//! Paper: `gelfond1991stable`
//! Oracle value: 0.0
//!
//! Status: CANDIDATE

use crate::breeds::breed::{BreedInput, CognitiveBreed};
use serde_json::json;
use std::collections::{BTreeSet, HashSet};

pub struct Asp;

struct Rule {
    head: String,
    pos_body: Vec<String>,
    neg_body: Vec<String>,
}

fn parse_rules(program: &[serde_json::Value]) -> Vec<Rule> {
    program
        .iter()
        .filter_map(|v| v.as_str())
        .map(|line| {
            let line = line.trim().trim_end_matches('.');
            if let Some((head_part, body_part)) = line.split_once(":-") {
                let head = head_part.trim().to_string();
                let mut pos_body = Vec::new();
                let mut neg_body = Vec::new();
                for lit in body_part.split(',') {
                    let lit = lit.trim();
                    if let Some(atom) = lit.strip_prefix("not ") {
                        neg_body.push(atom.trim().to_string());
                    } else {
                        pos_body.push(lit.to_string());
                    }
                }
                Rule {
                    head,
                    pos_body,
                    neg_body,
                }
            } else {
                Rule {
                    head: line.trim().to_string(),
                    pos_body: Vec::new(),
                    neg_body: Vec::new(),
                }
            }
        })
        .collect()
}

fn herbrand_base(rules: &[Rule]) -> Vec<String> {
    let mut atoms: HashSet<String> = HashSet::new();
    for r in rules {
        if !r.head.is_empty() {
            atoms.insert(r.head.clone());
        }
        for a in &r.pos_body {
            atoms.insert(a.clone());
        }
        for a in &r.neg_body {
            atoms.insert(a.clone());
        }
    }
    let mut v: Vec<String> = atoms.into_iter().collect();
    v.sort();
    v
}

fn gl_reduct<'a>(rules: &'a [Rule], model: &HashSet<String>) -> Vec<&'a Rule> {
    rules
        .iter()
        .filter(|r| r.neg_body.iter().all(|a| !model.contains(a)))
        .collect()
}

fn minimal_herbrand(definite: &[&Rule], base: &[String]) -> HashSet<String> {
    let mut current: HashSet<String> = HashSet::new();
    loop {
        let mut next = current.clone();
        for r in definite {
            if r.pos_body.iter().all(|a| current.contains(a)) {
                next.insert(r.head.clone());
            }
        }
        if next == current {
            break;
        }
        current = next;
    }
    current.retain(|a| base.contains(a));
    current
}

fn is_stable(candidate: &HashSet<String>, rules: &[Rule], base: &[String]) -> bool {
    let reduct = gl_reduct(rules, candidate);
    let minimal = minimal_herbrand(&reduct, base);
    *candidate == minimal
}

fn default_program() -> serde_json::Value {
    json!(["a :- not b.", "b :- not a."])
}

impl CognitiveBreed for Asp {
    fn breed_id(&self) -> &'static str {
        "asp"
    }

    fn run(&self, input: &BreedInput) -> Option<serde_json::Value> {
        let payload = if input
            .payload
            .as_object()
            .map(|m| m.is_empty())
            .unwrap_or(true)
        {
            json!({"program": default_program()})
        } else {
            input.payload.clone()
        };

        let prog_val = payload.get("program")?;
        let prog_arr = prog_val.as_array()?;
        let rules = parse_rules(prog_arr);
        let base = herbrand_base(&rules);
        let n = base.len();

        let mut models: Vec<Vec<String>> = Vec::new();

        for mask in 0u64..(1u64 << n) {
            let candidate: HashSet<String> = base
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, a)| a.clone())
                .collect();

            if is_stable(&candidate, &rules, &base) {
                let mut sorted: Vec<String> = candidate.into_iter().collect();
                sorted.sort();
                models.push(sorted);
            }
        }

        models.sort_by(|a, b| {
            let ka: BTreeSet<_> = a.iter().collect();
            let kb: BTreeSet<_> = b.iter().collect();
            ka.cmp(&kb)
        });

        Some(json!({"models": models}))
    }
}
