use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub enum ServerState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Reloading,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerDetails {
    pub state: ServerState,
    pub pid: Option<u32>,
    pub uptime_seconds: u64,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct ServerService {
    // Stateless for mocking purposes
}

impl ServerService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(&self, _host: String, _port: u16) -> Result<ServerDetails> {
        Ok(ServerDetails {
            state: ServerState::Starting,
            pid: Some(1234),
            uptime_seconds: 0,
        })
    }

    pub fn stop(&self, _force: bool) -> Result<ServerDetails> {
        Ok(ServerDetails {
            state: ServerState::Stopping,
            pid: None,
            uptime_seconds: 3600,
        })
    }

    pub fn status(&self) -> Result<ServerDetails> {
        Ok(ServerDetails {
            state: ServerState::Running,
            pid: Some(1234),
            uptime_seconds: 3600,
        })
    }

    pub fn reload(&self) -> Result<ServerDetails> {
        Ok(ServerDetails {
            state: ServerState::Reloading,
            pid: Some(1234),
            uptime_seconds: 3600,
        })
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[derive(Serialize)]
pub struct StartResult {
    pub details: ServerDetails,
}

#[verb("start")]
pub fn start(host: Option<String>, port: Option<u16>) -> Result<StartResult> {
    let service = ServerService::new();
    let host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = port.unwrap_or(8080);

    let details = service.start(host, port)?;
    Ok(StartResult { details })
}

#[derive(Serialize)]
pub struct StopResult {
    pub details: ServerDetails,
}

#[verb("stop")]
pub fn stop(force: Option<bool>) -> Result<StopResult> {
    let service = ServerService::new();
    let force = force.unwrap_or(false);

    let details = service.stop(force)?;
    Ok(StopResult { details })
}

#[derive(Serialize)]
pub struct StatusResult {
    pub details: ServerDetails,
}

#[verb("status")]
pub fn status() -> Result<StatusResult> {
    let service = ServerService::new();

    let details = service.status()?;
    Ok(StatusResult { details })
}

#[derive(Serialize)]
pub struct ReloadResult {
    pub details: ServerDetails,
}

#[verb("reload")]
pub fn reload() -> Result<ReloadResult> {
    let service = ServerService::new();

    let details = service.reload()?;
    Ok(ReloadResult { details })
}
