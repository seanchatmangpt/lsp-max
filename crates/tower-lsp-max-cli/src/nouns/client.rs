use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientState {
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    pub state: ClientState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub body: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct ClientService;

impl ClientService {
    fn load_mesh_json() -> serde_json::Value {
        let path = crate::nouns::get_state_path();
        if std::path::Path::new(&path).exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    return val;
                }
            }
        }
        serde_json::json!({
            "instances": {}
        })
    }

    fn save_mesh_json(val: &serde_json::Value) -> std::result::Result<(), String> {
        let path = crate::nouns::get_state_path();
        let content = serde_json::to_string_pretty(val).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn connect(id: String) -> std::result::Result<Client, String> {
        let mut mesh = Self::load_mesh_json();
        if !mesh.is_object() {
            mesh = serde_json::json!({});
        }
        let clients = mesh
            .as_object_mut()
            .unwrap()
            .entry("clients")
            .or_insert_with(|| serde_json::json!({}));
        clients[id.clone()] = serde_json::json!({
            "id": id.clone(),
            "state": "Connected",
            "messages": []
        });

        Self::save_mesh_json(&mesh)?;

        Ok(Client {
            id,
            state: ClientState::Connected,
        })
    }

    pub fn disconnect(id: String) -> std::result::Result<Client, String> {
        let mut mesh = Self::load_mesh_json();
        if !mesh.is_object() {
            mesh = serde_json::json!({});
        }
        let clients = mesh
            .as_object_mut()
            .unwrap()
            .entry("clients")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(client) = clients.get_mut(&id) {
            client["state"] = serde_json::json!("Disconnected");
        } else {
            clients[id.clone()] = serde_json::json!({
                "id": id.clone(),
                "state": "Disconnected",
                "messages": []
            });
        }

        Self::save_mesh_json(&mesh)?;

        Ok(Client {
            id,
            state: ClientState::Disconnected,
        })
    }

    pub fn send(id: String, message: String) -> std::result::Result<bool, String> {
        let mut mesh = Self::load_mesh_json();
        if !mesh.is_object() {
            mesh = serde_json::json!({});
        }
        let clients = mesh
            .as_object_mut()
            .unwrap()
            .entry("clients")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(client) = clients.get_mut(&id) {
            if let Some(msgs) = client.get_mut("messages").and_then(|m| m.as_array_mut()) {
                msgs.push(serde_json::json!(message));
            } else {
                client["messages"] = serde_json::json!([message]);
            }
        } else {
            clients[id.clone()] = serde_json::json!({
                "id": id.clone(),
                "state": "Connected",
                "messages": [message]
            });
        }

        Self::save_mesh_json(&mesh)?;
        Ok(true)
    }

    pub fn receive(id: String) -> std::result::Result<Message, String> {
        let mut mesh = Self::load_mesh_json();
        if !mesh.is_object() {
            mesh = serde_json::json!({});
        }
        let mut body = "No messages available".to_string();
        let clients = mesh
            .as_object_mut()
            .unwrap()
            .entry("clients")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(client) = clients.get_mut(&id) {
            if let Some(msgs) = client.get_mut("messages").and_then(|m| m.as_array_mut()) {
                if !msgs.is_empty() {
                    let popped = msgs.remove(0);
                    if let Some(s) = popped.as_str() {
                        body = s.to_string();
                    }
                }
            }
        }

        Self::save_mesh_json(&mesh)?;

        Ok(Message { body })
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[derive(Serialize)]
pub struct ConnectResult {
    pub client: Client,
}

#[verb("connect")]
pub fn connect(id: String) -> Result<ConnectResult> {
    let client = ClientService::connect(id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ConnectResult { client })
}

#[derive(Serialize)]
pub struct DisconnectResult {
    pub client: Client,
}

#[verb("disconnect")]
pub fn disconnect(id: String) -> Result<DisconnectResult> {
    let client = ClientService::disconnect(id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(DisconnectResult { client })
}

#[derive(Serialize)]
pub struct SendResult {
    pub success: bool,
}

#[verb("send")]
pub fn send(id: String, message: String) -> Result<SendResult> {
    let success = ClientService::send(id, message)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(SendResult { success })
}

#[derive(Serialize)]
pub struct ReceiveResult {
    pub message: Message,
}

#[verb("receive")]
pub fn receive(id: String) -> Result<ReceiveResult> {
    let message = ClientService::receive(id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ReceiveResult { message })
}
