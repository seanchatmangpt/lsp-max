//! Composition state and ComposedServer core (constructor + route_request).

use std::collections::HashMap;
use std::sync::Arc;

use lsp_types_max::Diagnostic;
use serde_json::{json, Value};
use tokio::sync::{broadcast, Mutex};

use crate::jsonrpc::{Error, Request, Result};
use crate::Client;

use super::capability_tracker::CapabilityTracker;
use super::edit_gate::TransactionEditGate;
use super::strategy::method_strategy;
use super::upstream::PersistentUpstream;
use super::version_tracker::DocumentVersionTracker;

/// State manager for language server composition.
#[derive(Debug)]
pub struct CompositionState {
    /// Tracks upstream capability changes and merges them.
    pub capability_tracker: CapabilityTracker,
    /// Tracks document open, change, and version states.
    pub doc_tracker: DocumentVersionTracker,
    /// Transactional guard to validate mutations.
    pub edit_gate: TransactionEditGate,
    /// Upstream request timeout config in milliseconds.
    pub upstream_timeout_ms: u64,
    /// Currently reported diagnostics mapped by source.
    pub diagnostics: HashMap<String, HashMap<String, Value>>,
    /// Monotonically increasing counter of requests.
    pub request_counter: u64,
    /// Trace history of requests.
    pub request_traces: Arc<std::sync::Mutex<Vec<Value>>>,
}

impl CompositionState {
    /// Creates a new composition state configured with the given upstreams.
    pub fn new(upstream_addresses: Vec<(String, String)>) -> Self {
        use super::strategy::UpstreamSource;
        let mut tracker = CapabilityTracker::new();
        for (id, addr) in upstream_addresses {
            tracker.add_source(UpstreamSource::new(id, addr));
        }
        let upstream_timeout_ms = std::env::var("TOWER_LSP_MAX_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(150);
        Self {
            capability_tracker: tracker,
            doc_tracker: DocumentVersionTracker::new(),
            edit_gate: TransactionEditGate::new(),
            upstream_timeout_ms,
            diagnostics: HashMap::new(),
            request_counter: 0,
            request_traces: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

/// Shared thread-safe handle to the composition state.
pub type SharedCompositionState = Arc<Mutex<CompositionState>>;

/// Composed LSP server that coordinates multiple upstream language servers.
#[derive(Debug)]
pub struct ComposedServer {
    pub(super) _client: Client,
    pub(super) state: SharedCompositionState,
    pub(super) upstreams: Arc<dashmap::DashMap<String, PersistentUpstream>>,
}

impl ComposedServer {
    /// Gets the shared composition state.
    pub fn state(&self) -> &SharedCompositionState {
        &self.state
    }

    /// Creates a new `ComposedServer` with a client handle and a list of upstream addresses.
    pub fn new(client: Client, upstream_addresses: Vec<(String, String)>) -> Self {
        let state = Arc::new(Mutex::new(CompositionState::new(
            upstream_addresses.clone(),
        )));
        let upstreams = Arc::new(dashmap::DashMap::new());
        let (unsolicited_tx, mut unsolicited_rx) = broadcast::channel(128);

        let _upstreams_clone = upstreams.clone();
        for (id, addr) in upstream_addresses {
            let id_clone = id.clone();
            let addr_clone = addr.clone();
            let unsolicited_tx = unsolicited_tx.clone();
            let upstreams_connections = upstreams.clone();
            tokio::spawn(async move {
                if let Some(conn) =
                    PersistentUpstream::connect(id_clone.clone(), addr_clone, 1000, unsolicited_tx)
                        .await
                {
                    upstreams_connections.insert(id_clone, conn);
                }
            });
        }

        let client_clone = client.clone();
        let state_clone = state.clone();
        let upstreams_connections_clone = upstreams.clone();
        tokio::spawn(async move {
            loop {
                match unsolicited_rx.recv().await {
                    Ok(notification) => {
                        let msg = notification.message;
                        let source_id = notification.source_id;

                        let method = match msg.get("method").and_then(|v| v.as_str()) {
                            Some(m) => m,
                            None => continue,
                        };

                        if method == "client/registerCapability" {
                            let mut valid_registrations = Vec::new();
                            if let Some(params) = msg.get("params") {
                                if let Some(registrations) =
                                    params.get("registrations").and_then(|v| v.as_array())
                                {
                                    let mut s = state_clone.lock().await;
                                    for reg in registrations {
                                        if let (Some(id), Some(m)) = (
                                            reg.get("id").and_then(|v| v.as_str()),
                                            reg.get("method").and_then(|v| v.as_str()),
                                        ) {
                                            let options = reg
                                                .get("registerOptions")
                                                .cloned()
                                                .unwrap_or(Value::Null);
                                            if s.capability_tracker
                                                .register_dynamic(id, m, &source_id, options)
                                            {
                                                valid_registrations.push(reg.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            let msg_id_opt = msg.get("id").cloned();
                            let client = client_clone.clone();
                            let upstreams_connections = upstreams_connections_clone.clone();
                            let source_id = source_id.clone();
                            if valid_registrations.is_empty() {
                                if let Some(msg_id) = msg_id_opt {
                                    tokio::spawn(async move {
                                        let resp = json!({ "jsonrpc": "2.0", "id": msg_id, "result": null });
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    });
                                }
                            } else {
                                let params = json!({ "registrations": valid_registrations });
                                tokio::spawn(async move {
                                    let req_id = client.next_request_id();
                                    let request =
                                        Request::build("client/registerCapability".to_string())
                                            .id(req_id.clone())
                                            .params(params)
                                            .finish();
                                    let mut client_clone = client.clone();
                                    use tower::Service;
                                    let resp_res = client_clone.call(request).await;
                                    if let Some(msg_id) = msg_id_opt {
                                        let resp = match resp_res {
                                            Ok(Some(response)) => {
                                                let (_, result) = response.into_parts();
                                                match result {
                                                    Ok(res) => {
                                                        json!({ "jsonrpc": "2.0", "id": msg_id, "result": res })
                                                    }
                                                    Err(err) => {
                                                        json!({ "jsonrpc": "2.0", "id": msg_id, "error": err })
                                                    }
                                                }
                                            }
                                            _ => {
                                                json!({ "jsonrpc": "2.0", "id": msg_id, "error": { "code": -32603, "message": "Internal error" } })
                                            }
                                        };
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    }
                                });
                            }
                        } else if method == "client/unregisterCapability" {
                            let mut valid_unregistrations = Vec::new();
                            if let Some(params) = msg.get("params") {
                                let unregs_opt = params
                                    .get("unregisterations")
                                    .and_then(|v| v.as_array())
                                    .or_else(|| {
                                        params.get("unregistrations").and_then(|v| v.as_array())
                                    });
                                if let Some(unregistrations) = unregs_opt {
                                    let mut s = state_clone.lock().await;
                                    for unreg in unregistrations {
                                        if let Some(id) = unreg.get("id").and_then(|v| v.as_str()) {
                                            if s.capability_tracker.unregister_dynamic(id) {
                                                valid_unregistrations.push(unreg.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            let msg_id_opt = msg.get("id").cloned();
                            let client = client_clone.clone();
                            let upstreams_connections = upstreams_connections_clone.clone();
                            let source_id = source_id.clone();
                            if valid_unregistrations.is_empty() {
                                if let Some(msg_id) = msg_id_opt {
                                    tokio::spawn(async move {
                                        let resp = json!({ "jsonrpc": "2.0", "id": msg_id, "result": null });
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    });
                                }
                            } else {
                                let params = json!({ "unregistrations": valid_unregistrations });
                                tokio::spawn(async move {
                                    let req_id = client.next_request_id();
                                    let request =
                                        Request::build("client/unregisterCapability".to_string())
                                            .id(req_id.clone())
                                            .params(params)
                                            .finish();
                                    let mut client_clone = client.clone();
                                    use tower::Service;
                                    let resp_res = client_clone.call(request).await;
                                    if let Some(msg_id) = msg_id_opt {
                                        let resp = match resp_res {
                                            Ok(Some(response)) => {
                                                let (_, result) = response.into_parts();
                                                match result {
                                                    Ok(res) => {
                                                        json!({ "jsonrpc": "2.0", "id": msg_id, "result": res })
                                                    }
                                                    Err(err) => {
                                                        json!({ "jsonrpc": "2.0", "id": msg_id, "error": err })
                                                    }
                                                }
                                            }
                                            _ => {
                                                json!({ "jsonrpc": "2.0", "id": msg_id, "error": { "code": -32603, "message": "Internal error" } })
                                            }
                                        };
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    }
                                });
                            }
                        } else if method == "textDocument/publishDiagnostics" {
                            if let Some(params) = msg.get("params") {
                                if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                                    let diagnostics_val =
                                        params.get("diagnostics").cloned().unwrap_or(json!([]));
                                    let (merged_diags, version) = {
                                        let mut s = state_clone.lock().await;
                                        s.diagnostics
                                            .entry(uri.to_string())
                                            .or_default()
                                            .insert(source_id.clone(), diagnostics_val);
                                        let mut flat_diags = Vec::new();
                                        if let Some(uri_diags) = s.diagnostics.get(uri) {
                                            for (src_id, diags_value) in uri_diags {
                                                if let Some(src) =
                                                    s.capability_tracker.sources.get(src_id)
                                                {
                                                    if src.is_routable() {
                                                        if let Some(diags_arr) =
                                                            diags_value.as_array()
                                                        {
                                                            for diag in diags_arr {
                                                                if src_id == "ggen-lsp" {
                                                                    let has_source_id = diag
                                                                        .get("data")
                                                                        .and_then(|data| {
                                                                            data.get("source_id")
                                                                        })
                                                                        .is_some();
                                                                    if !has_source_id {
                                                                        continue;
                                                                    }
                                                                }
                                                                let mut diag_clone = diag.clone();
                                                                if let Some(obj) =
                                                                    diag_clone.as_object_mut()
                                                                {
                                                                    obj.insert(
                                                                        "source".to_string(),
                                                                        Value::String(
                                                                            src_id.clone(),
                                                                        ),
                                                                    );
                                                                }
                                                                flat_diags.push(diag_clone);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        let version = s.doc_tracker.current_version(uri);
                                        (flat_diags, version)
                                    };
                                    if let Ok(diag_list) = serde_json::from_value::<Vec<Diagnostic>>(
                                        Value::Array(merged_diags),
                                    ) {
                                        if let Ok(parsed_uri) =
                                            <lsp_types_max::Uri as std::str::FromStr>::from_str(uri)
                                        {
                                            client_clone
                                                .publish_diagnostics(parsed_uri, diag_list, version)
                                                .await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Unsolicited message channel lagged by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        Self {
            _client: client,
            state,
            upstreams,
        }
    }

    pub(super) async fn route_request<P, R>(&self, method: &str, params: P) -> Result<Option<R>>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        println!("--- route_request [{}] start", method);
        let params_val = serde_json::to_value(params).unwrap_or(Value::Null);

        println!("--- route_request [{}] locking state at start", method);
        let (request_id, strategy_str, uri_opt, doc_version) = {
            let mut s = self.state.lock().await;
            println!(
                "--- route_request [{}] locked state at start inside block",
                method
            );
            s.request_counter += 1;
            let counter = s.request_counter;
            let strategy = method_strategy(method);
            let uri = params_val
                .get("textDocument")
                .and_then(|td| td.get("uri"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    params_val
                        .get("uri")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                });
            let ver = uri.as_ref().and_then(|u| s.doc_tracker.current_version(u));
            (
                format!("req_{}", counter),
                format!("{:?}", strategy),
                uri,
                ver,
            )
        };
        println!("--- route_request [{}] strategy: {}", method, strategy_str);

        let sources_contacted = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let sources_returned = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let source_health = Arc::new(std::sync::Mutex::new(std::collections::HashMap::<
            String,
            String,
        >::new()));
        let gate_outcome = Arc::new(std::sync::Mutex::new(None::<String>));
        let staleness_outcome = Arc::new(std::sync::Mutex::new(None::<String>));

        let res = self
            .route_request_internal(
                method,
                params_val,
                sources_contacted.clone(),
                sources_returned.clone(),
                source_health.clone(),
                gate_outcome.clone(),
                staleness_outcome.clone(),
            )
            .await;

        let final_response_val = match &res {
            Ok(Some(v)) => v.clone(),
            _ => Value::Null,
        };

        // Record the trace
        {
            let s = self.state.lock().await;
            let trace = json!({
                "request_id": request_id,
                "method": method,
                "document_uri": uri_opt,
                "document_version": doc_version,
                "strategy": strategy_str,
                "sources_contacted": *sources_contacted.lock().unwrap(),
                "sources_returned": *sources_returned.lock().unwrap(),
                "source_health": *source_health.lock().unwrap(),
                "gate_outcome": *gate_outcome.lock().unwrap(),
                "staleness_outcome": *staleness_outcome.lock().unwrap(),
                "final_response": final_response_val,
            });
            s.request_traces.lock().unwrap().push(trace);
        }

        match res {
            Ok(Some(v)) => match serde_json::from_value::<R>(v) {
                Ok(r) => Ok(Some(r)),
                Err(e) => Err(Error::invalid_params(format!(
                    "Failed to deserialize response: {}",
                    e
                ))),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
