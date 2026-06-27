use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::metamodel::{MessageDirection, MetaModel, OneOrManyTypes, Type};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageEntry {
    pub method: String,
    pub kind: String,
    pub direction: String,
    pub params_type: Option<String>,
    pub result_type: Option<String>,
}

fn direction_str(dir: &MessageDirection) -> String {
    match dir {
        MessageDirection::ClientToServer => "clientToServer".to_string(),
        MessageDirection::ServerToClient => "serverToClient".to_string(),
        MessageDirection::Both => "both".to_string(),
    }
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Reference { name } => name.clone(),
        Type::Base { name } => format!("{name:?}"),
        Type::Array { element } => format!("{}[]", type_name(element)),
        Type::Or { items } => items.iter().map(type_name).collect::<Vec<_>>().join(" | "),
        Type::And { items } => items.iter().map(type_name).collect::<Vec<_>>().join(" & "),
        Type::Tuple { items } => {
            format!(
                "[{}]",
                items.iter().map(type_name).collect::<Vec<_>>().join(", ")
            )
        }
        Type::Map { .. } => "map".to_string(),
        Type::StringLiteral { value } => format!("\"{value}\""),
        Type::IntegerLiteral { value } => value.to_string(),
        Type::BooleanLiteral { value } => value.to_string(),
        Type::Literal { .. } => "literal".to_string(),
    }
}

fn params_name(p: &Option<OneOrManyTypes>) -> Option<String> {
    match p {
        None => None,
        Some(OneOrManyTypes::One(t)) => Some(type_name(t)),
        Some(OneOrManyTypes::Many(ts)) => {
            Some(ts.iter().map(type_name).collect::<Vec<_>>().join(" | "))
        }
    }
}

/// Emit `lsp318_message_inventory.json` — flat list of all requests + notifications.
///
/// Output is a JSON array sorted deterministically by method name so
/// `git diff --exit-code` is meaningful on regeneration.
pub fn emit_message_inventory(model: &MetaModel) -> Result<String> {
    // Use BTreeMap keyed by method to deduplicate and sort.
    let mut map: BTreeMap<String, MessageEntry> = BTreeMap::new();

    for r in &model.requests {
        map.insert(
            r.method.clone(),
            MessageEntry {
                method: r.method.clone(),
                kind: "request".to_string(),
                direction: direction_str(&r.message_direction),
                params_type: params_name(&r.params),
                result_type: Some(type_name(&r.result)),
            },
        );
    }

    for n in &model.notifications {
        map.insert(
            n.method.clone(),
            MessageEntry {
                method: n.method.clone(),
                kind: "notification".to_string(),
                direction: direction_str(&n.message_direction),
                params_type: params_name(&n.params),
                result_type: None,
            },
        );
    }

    let entries: Vec<&MessageEntry> = map.values().collect();
    Ok(serde_json::to_string_pretty(&entries)?)
}
