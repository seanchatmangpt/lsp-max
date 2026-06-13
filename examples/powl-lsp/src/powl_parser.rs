// Thin JSON → wasm4pm_compat::powl::Powl parser using PowlBuilder.
// Structural invariants are enforced by PowlBuilder::build (delegates to Powl::validate).
// Discovery, replay, and conformance graduate to wasm4pm.

use serde_json::Value;
use wasm4pm_compat::powl::{Powl, PowlBuilder, PowlRefusal};

/// Parse a JSON value into a validated `Powl`.
pub fn parse_powl(v: &Value) -> Result<Powl, String> {
    let mut builder = PowlBuilder::new();
    let root_label = build_node(v, &mut builder)?;
    builder
        .root(&root_label)
        .build()
        .map_err(|r: PowlRefusal| format!("POWL structural law violated: {r}"))
}

fn build_node(v: &Value, builder: &mut PowlBuilder) -> Result<String, String> {
    let kind = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or("missing 'type' field")?;
    let label = v
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();

    match kind {
        "activity" => {
            // Take the builder by value; put it back via the mutable reference trick.
            replace_builder(builder, |b| b.atom(&label));
        }
        "silent" => {
            replace_builder(builder, |b| b.silent(&label));
        }
        "partial_order" => {
            let children = collect_children(v, builder)?;
            let edges = collect_order_edges(v)?;
            let edge_refs: Vec<(&str, &str)> = edges
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();
            let child_refs: Vec<&str> = children.iter().map(String::as_str).collect();
            replace_builder(builder, |b| {
                b.partial_order(&label, &child_refs, &edge_refs)
            });
        }
        "choice" => {
            let children = collect_children(v, builder)?;
            let child_refs: Vec<&str> = children.iter().map(String::as_str).collect();
            replace_builder(builder, |b| b.choice(&label, &child_refs));
        }
        "loop" => {
            let do_label = v
                .get("do")
                .ok_or_else(|| "loop missing 'do' field".to_string())
                .and_then(|n| build_node(n, builder))?;
            let redo_label = v.get("redo").map(|n| build_node(n, builder)).transpose()?;
            let redo_ref = redo_label.as_deref();
            replace_builder(builder, |b| b.loop_node(&label, &do_label, redo_ref));
        }
        "choice_graph" => {
            let children = collect_children(v, builder)?;
            let edges = collect_cg_edges(v)?;
            let edge_refs: Vec<(&str, &str)> = edges
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();
            let child_refs: Vec<&str> = children.iter().map(String::as_str).collect();
            replace_builder(builder, |b| b.choice_graph(&label, &child_refs, &edge_refs));
        }
        other => return Err(format!("unknown POWL node type '{other}'")),
    }

    Ok(label)
}

// PowlBuilder consumes self on every method. This helper swaps the builder
// out of the mutable reference, applies the transformation, and puts it back.
fn replace_builder(builder: &mut PowlBuilder, f: impl FnOnce(PowlBuilder) -> PowlBuilder) {
    let tmp = std::mem::take(builder);
    *builder = f(tmp);
}

fn collect_children(v: &Value, builder: &mut PowlBuilder) -> Result<Vec<String>, String> {
    let arr = v
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("missing 'nodes' array")?;
    arr.iter().map(|n| build_node(n, builder)).collect()
}

fn collect_order_edges(v: &Value) -> Result<Vec<(String, String)>, String> {
    let Some(arr) = v.get("edges").and_then(Value::as_array) else {
        return Ok(vec![]);
    };
    arr.iter()
        .map(|e| {
            let pair = e.as_array().ok_or("edge must be [from, to] array")?;
            if pair.len() != 2 {
                return Err("edge must have 2 elements".to_string());
            }
            let from = pair[0]
                .as_str()
                .ok_or("edge 'from' must be string")?
                .to_string();
            let to = pair[1]
                .as_str()
                .ok_or("edge 'to' must be string")?
                .to_string();
            Ok((from, to))
        })
        .collect()
}

fn collect_cg_edges(v: &Value) -> Result<Vec<(String, String)>, String> {
    collect_order_edges(v) // same wire format
}
