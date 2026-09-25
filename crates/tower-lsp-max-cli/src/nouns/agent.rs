use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// --- Domain Tier ---

#[derive(Debug, Clone, Serialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Planning,
    Halted,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentState {
    pub id: String,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentMessage {
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentPlan {
    pub steps: Vec<String>,
}

// --- Service Tier ---

pub struct AgentService;

impl AgentService {
    pub fn new() -> Self {
        Self
    }

    pub fn invoke(&self, id: String) -> AgentState {
        AgentState {
            id,
            status: AgentStatus::Working,
        }
    }

    pub fn chat(&self, _id: String, message: String) -> AgentMessage {
        AgentMessage {
            content: format!("Received: {}", message),
        }
    }

    pub fn plan(&self, _id: String) -> AgentPlan {
        AgentPlan {
            steps: vec!["Step 1".to_string(), "Step 2".to_string()],
        }
    }

    pub fn halt(&self, id: String) -> AgentState {
        AgentState {
            id,
            status: AgentStatus::Halted,
        }
    }
}

// --- CLI Tier ---

#[derive(Serialize)]
pub struct InvokeResult {
    pub state: AgentState,
}

#[verb("invoke")]
pub fn invoke(id: String) -> Result<InvokeResult> {
    let service = AgentService::new();
    let state = service.invoke(id);
    Ok(InvokeResult { state })
}

#[derive(Serialize)]
pub struct ChatResult {
    pub message: AgentMessage,
}

#[verb("chat")]
pub fn chat(id: String, message: String) -> Result<ChatResult> {
    let service = AgentService::new();
    let response = service.chat(id, message);
    Ok(ChatResult { message: response })
}

#[derive(Serialize)]
pub struct PlanResult {
    pub plan: AgentPlan,
}

#[verb("plan")]
pub fn plan(id: String) -> Result<PlanResult> {
    let service = AgentService::new();
    let plan = service.plan(id);
    Ok(PlanResult { plan })
}

#[derive(Serialize)]
pub struct HaltResult {
    pub state: AgentState,
}

#[verb("halt")]
pub fn halt(id: String) -> Result<HaltResult> {
    let service = AgentService::new();
    let state = service.halt(id);
    Ok(HaltResult { state })
}
