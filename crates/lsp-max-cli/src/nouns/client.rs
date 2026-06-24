use clap_noun_verb::error::NounVerbError;
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

/// Register a client as Connected and persist its state.
#[verb("connect")]
pub fn connect(id: String) -> Result<ConnectResult> {
    let client = ClientService::connect(id).map_err(NounVerbError::execution_error)?;
    Ok(ConnectResult { client })
}

#[derive(Serialize)]
pub struct DisconnectResult {
    pub client: Client,
}

/// Set a client to Disconnected, creating the record if absent.
#[verb("disconnect")]
pub fn disconnect(id: String) -> Result<DisconnectResult> {
    let client = ClientService::disconnect(id).map_err(NounVerbError::execution_error)?;
    Ok(DisconnectResult { client })
}

#[derive(Serialize)]
pub struct SendResult {
    pub success: bool,
}

/// Append a message to a client's inbound queue.
#[verb("send")]
pub fn send(id: String, message: String) -> Result<SendResult> {
    let success = ClientService::send(id, message).map_err(NounVerbError::execution_error)?;
    Ok(SendResult { success })
}

#[derive(Serialize)]
pub struct ReceiveResult {
    pub message: Message,
}

/// Pop and return the first message from a client's queue.
#[verb("receive")]
pub fn receive(id: String) -> Result<ReceiveResult> {
    let message = ClientService::receive(id).map_err(NounVerbError::execution_error)?;
    Ok(ReceiveResult { message })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// RAII guard — sets LSP_MAX_STATE_PATH to a fresh temp file, restores on drop.
    struct StateGuard {
        _tmp: tempfile::NamedTempFile,
        prev: Option<String>,
    }

    impl StateGuard {
        fn new() -> Self {
            let tmp = tempfile::NamedTempFile::new().unwrap();
            let path = tmp.path().to_str().unwrap().to_string();
            let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
            // SAFETY: under TEST_ENV_LOCK held by the caller.
            unsafe { std::env::set_var("LSP_MAX_STATE_PATH", &path) };
            Self { _tmp: tmp, prev }
        }
    }

    impl Drop for StateGuard {
        fn drop(&mut self) {
            // SAFETY: restoring env state under TEST_ENV_LOCK.
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var("LSP_MAX_STATE_PATH", v),
                    None => std::env::remove_var("LSP_MAX_STATE_PATH"),
                }
            }
        }
    }

    // --- connect ---

    #[test]
    fn connect_returns_connected_state() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let client = ClientService::connect("client-1".to_string()).unwrap();
        assert_eq!(client.id, "client-1");
        assert!(matches!(client.state, ClientState::Connected));
    }

    // --- disconnect ---

    #[test]
    fn disconnect_returns_disconnected_state() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let _ = ClientService::connect("client-2".to_string()).unwrap();
        let client = ClientService::disconnect("client-2".to_string()).unwrap();
        assert_eq!(client.id, "client-2");
        assert!(matches!(client.state, ClientState::Disconnected));
    }

    #[test]
    fn disconnect_unknown_client_creates_disconnected_record() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        // Disconnect without prior connect must succeed — creates a Disconnected entry.
        let client = ClientService::disconnect("never-connected".to_string()).unwrap();
        assert!(matches!(client.state, ClientState::Disconnected));
    }

    // --- send ---

    #[test]
    fn send_returns_true() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let result = ClientService::send("client-3".to_string(), "hello".to_string()).unwrap();
        assert!(result);
    }

    // --- receive ---

    #[test]
    fn receive_empty_queue_returns_default_message() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let msg = ClientService::receive("client-4".to_string()).unwrap();
        assert_eq!(msg.body, "No messages available");
    }

    #[test]
    fn send_then_receive_pops_the_message() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let _ = ClientService::connect("client-5".to_string()).unwrap();
        ClientService::send("client-5".to_string(), "ping".to_string()).unwrap();
        let msg = ClientService::receive("client-5".to_string()).unwrap();
        assert_eq!(
            msg.body, "ping",
            "receive must pop and return the sent message"
        );
    }

    #[test]
    fn second_receive_returns_default_after_queue_drained() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let _ = ClientService::connect("client-6".to_string()).unwrap();
        ClientService::send("client-6".to_string(), "once".to_string()).unwrap();
        let _ = ClientService::receive("client-6".to_string()).unwrap();
        let msg = ClientService::receive("client-6".to_string()).unwrap();
        assert_eq!(msg.body, "No messages available");
    }
}
