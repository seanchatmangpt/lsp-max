use crate::breeds::breed::{BreedInput, CognitiveBreed};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct FramesInheritance;

const MAX_DEPTH: usize = 20;

fn lookup_slot<'a>(
    frames: &'a HashMap<String, HashMap<String, String>>,
    start: &str,
    slot: &str,
) -> Option<Value> {
    let mut current = start.to_string();
    let mut depth: usize = 0;
    let mut visited = Vec::with_capacity(MAX_DEPTH);

    loop {
        if visited.contains(&current) || depth > MAX_DEPTH {
            return Some(json!({"error": "cycle detected in isa chain"}));
        }
        visited.push(current.clone());

        let frame = frames.get(&current)?;

        if let Some(value) = frame.get(slot) {
            let inherited_from = if current == start {
                start.to_string()
            } else {
                current.clone()
            };
            return Some(json!({
                "value": value,
                "inherited_from": inherited_from,
                "depth": depth
            }));
        }

        match frame.get("isa") {
            Some(parent) => {
                current = parent.clone();
                depth += 1;
            }
            None => return Some(json!({"error": "slot not found"})),
        }
    }
}

fn parse_frames(val: &Value) -> Option<HashMap<String, HashMap<String, String>>> {
    let obj = val.as_object()?;
    let mut frames = HashMap::new();
    for (frame_name, slots_val) in obj {
        let slots_obj = slots_val.as_object()?;
        let mut slots = HashMap::new();
        for (k, v) in slots_obj {
            if let Some(s) = v.as_str() {
                slots.insert(k.clone(), s.to_string());
            }
        }
        frames.insert(frame_name.clone(), slots);
    }
    Some(frames)
}

impl CognitiveBreed for FramesInheritance {
    fn breed_id(&self) -> &'static str {
        "frames_inheritance"
    }

    fn run(&self, input: &BreedInput) -> Option<Value> {
        let frames_val = input.get("frames")?;
        let query = input.get("query")?;
        let frame_name = query.get("frame")?.as_str()?;
        let slot = query.get("slot")?.as_str()?;

        let frames = parse_frames(frames_val)?;
        Some(lookup_slot(&frames, frame_name, slot)
            .unwrap_or_else(|| json!({"error": "frame not found"})))
    }
}
