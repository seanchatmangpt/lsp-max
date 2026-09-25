use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub enum ClientState {
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, Serialize)]
pub struct Client {
    pub id: String,
    pub state: ClientState,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub body: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct ClientService;

impl ClientService {
    pub fn connect(id: String) -> Client {
        Client {
            id,
            state: ClientState::Connected,
        }
    }

    pub fn disconnect(id: String) -> Client {
        Client {
            id,
            state: ClientState::Disconnected,
        }
    }

    pub fn send(_id: String, _message: String) -> bool {
        true
    }

    pub fn receive(_id: String) -> Message {
        Message {
            body: "mock_response".to_string(),
        }
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
    let client = ClientService::connect(id);
    Ok(ConnectResult { client })
}

#[derive(Serialize)]
pub struct DisconnectResult {
    pub client: Client,
}

#[verb("disconnect")]
pub fn disconnect(id: String) -> Result<DisconnectResult> {
    let client = ClientService::disconnect(id);
    Ok(DisconnectResult { client })
}

#[derive(Serialize)]
pub struct SendResult {
    pub success: bool,
}

#[verb("send")]
pub fn send(id: String, message: String) -> Result<SendResult> {
    let success = ClientService::send(id, message);
    Ok(SendResult { success })
}

#[derive(Serialize)]
pub struct ReceiveResult {
    pub message: Message,
}

#[verb("receive")]
pub fn receive(id: String) -> Result<ReceiveResult> {
    let message = ClientService::receive(id);
    Ok(ReceiveResult { message })
}
