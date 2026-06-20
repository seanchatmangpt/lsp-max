use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::fs;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct StreamStatusResult {
    pub subscriptions: Vec<StreamSubscriptionInfo>,
    pub event_bus_capacity: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamSubscriptionInfo {
    pub subscription_id: String,
    pub event_kinds: Vec<String>,
    pub uri_filter: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamSubscribeResult {
    pub subscription_id: String,
    pub event_kinds: Vec<String>,
    pub status: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamUnsubscribeResult {
    pub subscription_id: String,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

/// Stream subscriptions are stored in the mesh state JSON.
pub struct StreamService;

impl StreamService {
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

    pub fn status(&self) -> StreamStatusResult {
        let state = Self::load_state();
        let subs: Vec<StreamSubscriptionInfo> = state
            .get("stream_subscriptions")
            .and_then(|s| s.as_object())
            .map(|m| {
                m.iter()
                    .map(|(id, v)| StreamSubscriptionInfo {
                        subscription_id: id.clone(),
                        event_kinds: v
                            .get("event_kinds")
                            .and_then(|e| e.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        uri_filter: v
                            .get("uri_filter")
                            .and_then(|u| u.as_str())
                            .map(str::to_string),
                        status: "CANDIDATE".into(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        StreamStatusResult {
            subscriptions: subs,
            event_bus_capacity: 256,
            status: "CANDIDATE".into(),
        }
    }

    pub fn subscribe(
        &self,
        id: &str,
        kinds: Vec<String>,
        uri: Option<String>,
    ) -> Result<StreamSubscribeResult> {
        let mut state = Self::load_state();
        let mut new_subs = state["stream_subscriptions"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        new_subs.insert(
            id.to_string(),
            serde_json::json!({
                "event_kinds": kinds,
                "uri_filter": uri,
                "status": "CANDIDATE",
            }),
        );
        state["stream_subscriptions"] = serde_json::Value::Object(new_subs);
        Self::save_state(&state)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e))?;

        Ok(StreamSubscribeResult {
            subscription_id: id.to_string(),
            event_kinds: kinds,
            status: "CANDIDATE".into(),
            note: "max/stream subscription registered — events delivered via LSP notifications when server is running".into(),
        })
    }

    pub fn unsubscribe(&self, id: &str) -> Result<StreamUnsubscribeResult> {
        let mut state = Self::load_state();
        if let Some(subs) = state["stream_subscriptions"].as_object_mut() {
            subs.remove(id);
        }
        Self::save_state(&state)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e))?;
        Ok(StreamUnsubscribeResult {
            subscription_id: id.to_string(),
            status: "CANDIDATE".into(),
        })
    }
}

impl Default for StreamService {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[verb("status")]
pub fn status() -> Result<StreamStatusResult> {
    Ok(StreamService::new().status())
}

#[verb("subscribe")]
pub fn subscribe(
    id: String,
    kinds: Option<String>,
    uri: Option<String>,
) -> Result<StreamSubscribeResult> {
    let kinds = kinds.unwrap_or_else(|| "Diagnostic,LawViolation".to_string());
    let kind_list: Vec<String> = kinds.split(',').map(str::trim).map(str::to_string).collect();
    StreamService::new().subscribe(&id, kind_list, uri)
}

#[verb("unsubscribe")]
pub fn unsubscribe(id: String) -> Result<StreamUnsubscribeResult> {
    StreamService::new().unsubscribe(&id)
}

// ==========================================
// 4. Tests
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nouns::TEST_ENV_LOCK;

    #[test]
    fn status_returns_candidate_with_no_state_file() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let svc = StreamService::new();
        let result = svc.status();
        assert_eq!(result.status, "CANDIDATE");
        assert_eq!(result.event_bus_capacity, 256);
    }

    #[test]
    fn subscribe_and_unsubscribe_roundtrip() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("LSP_MAX_STATE_PATH", &path);

        let svc = StreamService::new();
        let sub = svc
            .subscribe(
                "test-sub-1",
                vec!["Diagnostic".into(), "LawViolation".into()],
                None,
            )
            .expect("subscribe should not fail");
        assert_eq!(sub.subscription_id, "test-sub-1");
        assert_eq!(sub.status, "CANDIDATE");

        let status = svc.status();
        assert_eq!(status.subscriptions.len(), 1);
        assert_eq!(status.subscriptions[0].subscription_id, "test-sub-1");

        let unsub = svc.unsubscribe("test-sub-1").expect("unsubscribe should not fail");
        assert_eq!(unsub.subscription_id, "test-sub-1");

        let status_after = svc.status();
        assert!(status_after.subscriptions.is_empty());

        std::env::remove_var("LSP_MAX_STATE_PATH");
    }
}
