use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::fs;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct IntentDeclareOutput {
    pub intent_id: String,
    pub outcome: String,
    pub gate_open: bool,
    pub law_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntentListOutput {
    pub intents: Vec<serde_json::Value>,
    pub total: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntentRevokeOutput {
    pub intent_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntentCheckOutput {
    pub intent_id: String,
    pub gate_open: bool,
    pub outcome: String,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct IntentService;

impl IntentService {
    pub fn new() -> Self {
        Self
    }

    fn state_path() -> String {
        std::env::var("LSP_MAX_STATE_PATH").unwrap_or_else(|_| ".mesh_state.json".to_string())
    }

    fn load_state() -> serde_json::Value {
        let path = Self::state_path();
        if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        }
    }

    fn save_state(val: &serde_json::Value) -> std::result::Result<(), String> {
        let path = Self::state_path();
        let content = serde_json::to_string_pretty(val).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    fn load_intents() -> serde_json::Value {
        let state = Self::load_state();
        state
            .get("intents")
            .cloned()
            .unwrap_or(serde_json::json!({}))
    }

    fn save_intents(intents: serde_json::Value) -> std::result::Result<(), String> {
        let mut state = Self::load_state();
        state["intents"] = intents;
        Self::save_state(&state)
    }

    fn validate_kind(kind: &str, target: &str) -> (String, bool) {
        if target.contains("tower-lsp") || target.contains("tower_lsp") {
            return (
                "Blocked: LawViolation — tower-lsp forbidden reference".into(),
                false,
            );
        }
        if kind == "ShellExec" && target.contains("--no-verify") {
            return (
                "Blocked: LawViolation — --no-verify bypasses law hooks".into(),
                false,
            );
        }
        ("Cleared".into(), true)
    }

    pub fn declare(
        &self,
        id: &str,
        kind: &str,
        target: &str,
        rationale: Option<&str>,
    ) -> Result<IntentDeclareOutput> {
        let (outcome, gate_open) = Self::validate_kind(kind, target);
        let law_status = if gate_open {
            "CANDIDATE".into()
        } else {
            "REFUSED".into()
        };
        let mut intents = Self::load_intents().as_object().cloned().unwrap_or_default();
        intents.insert(
            id.to_string(),
            serde_json::json!({
                "kind": kind,
                "target": target,
                "rationale": rationale.unwrap_or(""),
                "outcome": outcome,
                "gate_open": gate_open,
                "law_status": law_status,
            }),
        );
        Self::save_intents(serde_json::Value::Object(intents))
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e))?;
        Ok(IntentDeclareOutput {
            intent_id: id.to_string(),
            outcome,
            gate_open,
            law_status,
        })
    }

    pub fn list(&self, filter: Option<&str>) -> IntentListOutput {
        let intents = Self::load_intents();
        let entries: Vec<serde_json::Value> = intents
            .as_object()
            .map(|m| {
                m.iter()
                    .filter(|(_, v)| {
                        filter
                            .map(|f| {
                                v.get("outcome")
                                    .and_then(|o| o.as_str())
                                    .map(|o| o.starts_with(f))
                                    .unwrap_or(false)
                            })
                            .unwrap_or(true)
                    })
                    .map(|(id, v)| {
                        let mut e = v.clone();
                        e["intent_id"] = serde_json::json!(id);
                        e
                    })
                    .collect()
            })
            .unwrap_or_default();
        let total = entries.len();
        IntentListOutput {
            intents: entries,
            total,
            status: "CANDIDATE".into(),
        }
    }

    pub fn revoke(&self, id: &str) -> Result<IntentRevokeOutput> {
        let mut intents = Self::load_intents().as_object().cloned().unwrap_or_default();
        intents.remove(id);
        Self::save_intents(serde_json::Value::Object(intents))
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e))?;
        Ok(IntentRevokeOutput {
            intent_id: id.to_string(),
            status: "REVOKED".into(),
        })
    }

    pub fn check(&self, id: &str) -> IntentCheckOutput {
        let intents = Self::load_intents();
        match intents.get(id) {
            None => IntentCheckOutput {
                intent_id: id.to_string(),
                gate_open: false,
                outcome: "NotFound".into(),
                status: "UNKNOWN".into(),
            },
            Some(v) => IntentCheckOutput {
                intent_id: id.to_string(),
                gate_open: v
                    .get("gate_open")
                    .and_then(|g| g.as_bool())
                    .unwrap_or(false),
                outcome: v
                    .get("outcome")
                    .and_then(|o| o.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                status: v
                    .get("law_status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
            },
        }
    }
}

impl Default for IntentService {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

/// Declare a pre-flight intent for the named action kind and target.
/// Law-violating targets (tower-lsp references, --no-verify flags) are blocked immediately.
#[verb("declare")]
pub fn declare(
    id: String,
    kind: String,
    target: String,
    rationale: Option<String>,
) -> Result<IntentDeclareOutput> {
    IntentService::new().declare(&id, &kind, &target, rationale.as_deref())
}

/// List declared intents, optionally filtered by outcome prefix ("Cleared", "Blocked").
#[verb("list")]
pub fn list(filter: Option<String>) -> Result<IntentListOutput> {
    Ok(IntentService::new().list(filter.as_deref()))
}

/// Revoke a previously declared intent by ID.
#[verb("revoke")]
pub fn revoke(id: String) -> Result<IntentRevokeOutput> {
    IntentService::new().revoke(&id)
}

/// Check the current gate-open status of a declared intent.
#[verb("check")]
pub fn check(id: String) -> Result<IntentCheckOutput> {
    Ok(IntentService::new().check(&id))
}

// ==========================================
// 4. Tests
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nouns::TEST_ENV_LOCK;

    #[test]
    fn declare_cleared_for_clean_target() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("LSP_MAX_STATE_PATH", &path);

        let svc = IntentService::new();
        let result = svc
            .declare("intent-1", "FileWrite", "src/foo.rs", Some("add feature"))
            .expect("declare should not fail");
        assert_eq!(result.intent_id, "intent-1");
        assert_eq!(result.outcome, "Cleared");
        assert!(result.gate_open);
        assert_eq!(result.law_status, "CANDIDATE");

        std::env::remove_var("LSP_MAX_STATE_PATH");
    }

    #[test]
    fn declare_blocked_for_tower_lsp_uri() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("LSP_MAX_STATE_PATH", &path);

        let svc = IntentService::new();
        let result = svc
            .declare("intent-2", "FileWrite", "src/tower-lsp/foo.rs", None)
            .expect("declare should not fail");
        assert!(!result.gate_open);
        assert!(result.outcome.starts_with("Blocked"));
        assert_eq!(result.law_status, "REFUSED");

        std::env::remove_var("LSP_MAX_STATE_PATH");
    }

    #[test]
    fn declare_blocked_for_no_verify_shell() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("LSP_MAX_STATE_PATH", &path);

        let svc = IntentService::new();
        let result = svc
            .declare(
                "intent-3",
                "ShellExec",
                "git commit --no-verify -m msg",
                None,
            )
            .expect("declare should not fail");
        assert!(!result.gate_open);
        assert!(result.outcome.starts_with("Blocked"));

        std::env::remove_var("LSP_MAX_STATE_PATH");
    }

    #[test]
    fn list_and_revoke_roundtrip() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("LSP_MAX_STATE_PATH", &path);

        let svc = IntentService::new();
        svc.declare("intent-a", "LspCall", "textDocument/hover", None)
            .expect("declare should not fail");
        svc.declare("intent-b", "GitPush", "main", None)
            .expect("declare should not fail");

        let listed = svc.list(None);
        assert_eq!(listed.total, 2);

        let cleared = svc.list(Some("Cleared"));
        assert_eq!(cleared.total, 2);

        svc.revoke("intent-a").expect("revoke should not fail");
        let after_revoke = svc.list(None);
        assert_eq!(after_revoke.total, 1);

        std::env::remove_var("LSP_MAX_STATE_PATH");
    }

    #[test]
    fn check_returns_unknown_for_missing_intent() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("LSP_MAX_STATE_PATH", &path);

        let svc = IntentService::new();
        let result = svc.check("nonexistent-intent");
        assert_eq!(result.status, "UNKNOWN");
        assert_eq!(result.outcome, "NotFound");
        assert!(!result.gate_open);

        std::env::remove_var("LSP_MAX_STATE_PATH");
    }
}
