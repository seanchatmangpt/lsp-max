use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Reloading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDetails {
    pub state: ServerState,
    pub pid: Option<u32>,
    pub uptime_seconds: u64,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct ServerService;

impl ServerService {
    pub fn new() -> Self {
        Self {}
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn is_pid_running(pid: u32) -> bool {
        let mut cmd = Command::new("kill");
        cmd.arg("-0").arg(pid.to_string());
        if let Ok(status) = cmd.status() {
            status.success()
        } else {
            false
        }
    }

    fn kill_pid(pid: u32, force: bool) {
        let mut cmd = Command::new("kill");
        if force {
            cmd.arg("-9");
        }
        cmd.arg(pid.to_string());
        let _ = cmd.status();
    }

    fn spawn_server_process() -> std::io::Result<u32> {
        let child = Command::new("sleep")
            .arg("3600")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(child.id())
    }

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

    pub fn start(&self, host: String, port: u16) -> Result<ServerDetails> {
        let mut mesh = Self::load_mesh_json();

        if let Some(srv) = mesh.get("server") {
            if let Some(pid) = srv.get("pid").and_then(|p| p.as_u64()) {
                if Self::is_pid_running(pid as u32) {
                    let start_time = srv.get("start_time").and_then(|t| t.as_u64()).unwrap_or(0);
                    let uptime = Self::current_timestamp().saturating_sub(start_time);
                    return Ok(ServerDetails {
                        state: ServerState::Running,
                        pid: Some(pid as u32),
                        uptime_seconds: uptime,
                    });
                }
            }
        }

        let pid = Self::spawn_server_process()
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
        let now = Self::current_timestamp();

        mesh["server"] = serde_json::json!({
            "state": "Running",
            "pid": pid,
            "start_time": now,
            "host": host,
            "port": port
        });

        Self::save_mesh_json(&mesh)
            .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

        Ok(ServerDetails {
            state: ServerState::Starting,
            pid: Some(pid),
            uptime_seconds: 0,
        })
    }

    pub fn stop(&self, force: bool) -> Result<ServerDetails> {
        let mut mesh = Self::load_mesh_json();

        let mut target_pid = None;
        let mut start_time = 0;

        if let Some(srv) = mesh.get("server") {
            if let Some(pid) = srv.get("pid").and_then(|p| p.as_u64()) {
                target_pid = Some(pid as u32);
                start_time = srv.get("start_time").and_then(|t| t.as_u64()).unwrap_or(0);
            }
        }

        let uptime = if start_time > 0 {
            Self::current_timestamp().saturating_sub(start_time)
        } else {
            0
        };

        if let Some(pid) = target_pid {
            if Self::is_pid_running(pid) {
                Self::kill_pid(pid, force);
            }
        }

        mesh["server"] = serde_json::json!({
            "state": "Stopped",
            "pid": null,
            "start_time": 0
        });

        Self::save_mesh_json(&mesh)
            .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

        Ok(ServerDetails {
            state: ServerState::Stopped,
            pid: None,
            uptime_seconds: uptime,
        })
    }

    pub fn status(&self) -> Result<ServerDetails> {
        let mesh = Self::load_mesh_json();

        if let Some(srv) = mesh.get("server") {
            if let Some(pid) = srv.get("pid").and_then(|p| p.as_u64()) {
                if Self::is_pid_running(pid as u32) {
                    let start_time = srv.get("start_time").and_then(|t| t.as_u64()).unwrap_or(0);
                    let uptime = Self::current_timestamp().saturating_sub(start_time);
                    return Ok(ServerDetails {
                        state: ServerState::Running,
                        pid: Some(pid as u32),
                        uptime_seconds: uptime,
                    });
                }
            }
        }

        Ok(ServerDetails {
            state: ServerState::Stopped,
            pid: None,
            uptime_seconds: 0,
        })
    }

    pub fn reload(&self) -> Result<ServerDetails> {
        let status = self.status()?;
        if let Some(pid) = status.pid {
            Self::kill_pid(pid, true);
        }
        self.start("127.0.0.1".to_string(), 8080)
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
