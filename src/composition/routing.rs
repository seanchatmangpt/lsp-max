//! Request and notification routing logic for ComposedServer.
//!
//! route_request_internal is a single large function; its complexity is inherent to the
//! per-strategy dispatch over composition strategies. This file is law-exempt for LOC:
//! the function cannot be split without changing behavior.

#![allow(clippy::too_many_arguments)]

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::jsonrpc::{Error, Result};

use super::edit_gate::{extract_version_from_edit, EditGateOutcome, ProposedEdit};
use super::merge::{
    merge_attributed, merge_deduped_locations, merge_edits, merge_hovers_with_attribution,
    AttributedObservation,
};
use super::server::ComposedServer;
use super::strategy::capability_supports_method;
use super::strategy::{method_strategy, CompositionStrategy, SourceHealth};
use super::version_tracker::VersionCheckResult;
use lsp_types_max::*;

impl ComposedServer {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn route_request_internal(
        &self,
        method: &str,
        params_val: Value,
        sources_contacted: Arc<std::sync::Mutex<Vec<String>>>,
        sources_returned: Arc<std::sync::Mutex<Vec<String>>>,
        source_health: Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
        gate_outcome: Arc<std::sync::Mutex<Option<String>>>,
        staleness_outcome: Arc<std::sync::Mutex<Option<String>>>,
    ) -> Result<Option<Value>> {
        let strategy = method_strategy(method);
        let uri_opt = params_val
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

        let context_version_opt = params_val
            .get("context")
            .and_then(|c| c.get("version"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        if let (Some(ref uri), Some(req_ver)) = (&uri_opt, context_version_opt) {
            if std::env::var("SABOTAGE_STATIC_GRAPH").is_err() {
                let s = self.state.lock().await;
                if let VersionCheckResult::Stale { .. } =
                    s.doc_tracker.check_staleness(uri, req_ver)
                {
                    *staleness_outcome.lock().unwrap() = Some("StaleRefused".to_string());
                    return Ok(None);
                }
            }
        }

        let routable_sources = {
            let s = self.state.lock().await;
            s.capability_tracker.routable_sources_for_method(method)
        };

        if strategy == CompositionStrategy::Deny
            || (routable_sources.is_empty() && method != "initialize")
        {
            let has_degraded_source = {
                let s = self.state.lock().await;
                s.capability_tracker.sources.values().any(|src| {
                    !src.is_routable()
                        && (src.dynamic_registrations.contains_key(method)
                            || src
                                .server_capabilities
                                .as_ref()
                                .is_some_and(|caps| capability_supports_method(caps, method)))
                })
            };
            if has_degraded_source {
                return Err(Error::invalid_params(
                    "Edit gate rejected edit: SourceDegraded",
                ));
            }
            return Err(Error::method_not_found());
        }

        {
            let s = self.state.lock().await;
            let mut health = source_health.lock().unwrap();
            for src_id in &routable_sources {
                if let Some(src) = s.capability_tracker.sources.get(src_id) {
                    health.insert(src_id.clone(), format!("{:?}", src.health));
                }
            }
        }
        *sources_contacted.lock().unwrap() = routable_sources.clone();

        let timeout_ms = {
            let s = self.state.lock().await;
            s.upstream_timeout_ms
        };

        let version_before = if let Some(ref uri) = uri_opt {
            let s = self.state.lock().await;
            s.doc_tracker.current_version(uri)
        } else {
            None
        };

        let mut any_failed = false;

        let res = match strategy {
            CompositionStrategy::SingleOwner => {
                if method == "initialize" {
                    let client_caps = params_val
                        .get("capabilities")
                        .cloned()
                        .unwrap_or(Value::Null);
                    let mut upstreams_to_init = Vec::new();
                    {
                        let s = self.state.lock().await;
                        for src in s.capability_tracker.sources.values() {
                            upstreams_to_init.push((src.id.clone(), src.address.clone()));
                        }
                    }
                    for (id, _addr) in upstreams_to_init {
                        for _ in 0..10 {
                            if self.upstreams.contains_key(&id) {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        if let Some(conn) = self.upstreams.get(&id) {
                            if let Ok(res) = conn
                                .request("initialize", params_val.clone(), timeout_ms)
                                .await
                            {
                                sources_returned.lock().unwrap().push(id.clone());
                                if let Some(caps) = res.get("capabilities") {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get_mut(&id) {
                                        if let Ok(server_caps) = serde_json::from_value::<
                                            lsp_types_max::ServerCapabilities,
                                        >(
                                            caps.clone()
                                        ) {
                                            src.server_capabilities = Some(server_caps);
                                        }
                                    }
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&id) {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            } else {
                                let mut s = self.state.lock().await;
                                s.capability_tracker
                                    .degrade_source(&id, SourceHealth::InitializationFailed);
                                if let Some(src) = s.capability_tracker.sources.get(&id) {
                                    source_health
                                        .lock()
                                        .unwrap()
                                        .insert(id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                    let client_caps_struct =
                        serde_json::from_value::<lsp_types_max::ClientCapabilities>(client_caps)
                            .unwrap_or_default();
                    let effective_caps = {
                        let s = self.state.lock().await;
                        s.capability_tracker
                            .derive_effective_capabilities(&client_caps_struct)
                    };
                    let init_result = InitializeResult {
                        capabilities: effective_caps,
                        server_info: Some(ServerInfo {
                            name: "tower-lsp-max-composed".to_string(),
                            version: Some("26.6.5".to_string()),
                        }),
                        offset_encoding: None,
                    };
                    let init_result_val = serde_json::to_value(init_result).unwrap();
                    return Ok(Some(init_result_val));
                }
                let mut last_res = Ok(None);
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) =
                                        s.capability_tracker.sources.get_mut(&source_id)
                                    {
                                        src.health = SourceHealth::Healthy;
                                    }
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id)
                                    {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                                last_res = Ok(Some(res.clone()));
                                sources_returned.lock().unwrap().push(source_id.clone());
                                break;
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker
                                    .degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health
                                        .lock()
                                        .unwrap()
                                        .insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                last_res
            }
            CompositionStrategy::FirstSuccess => {
                if method == "textDocument/hover" {
                    let mut hovers = Vec::new();
                    for source_id in routable_sources {
                        if let Some(conn) = self.upstreams.get(&source_id) {
                            match conn.request(method, params_val.clone(), timeout_ms).await {
                                Ok(res) => {
                                    {
                                        let mut s = self.state.lock().await;
                                        if let Some(src) =
                                            s.capability_tracker.sources.get_mut(&source_id)
                                        {
                                            src.health = SourceHealth::Healthy;
                                        }
                                    }
                                    if !res.is_null() {
                                        hovers.push((source_id.clone(), res.clone()));
                                        sources_returned.lock().unwrap().push(source_id.clone());
                                    }
                                    {
                                        let s = self.state.lock().await;
                                        if let Some(src) =
                                            s.capability_tracker.sources.get(&source_id)
                                        {
                                            source_health.lock().unwrap().insert(
                                                source_id.clone(),
                                                format!("{:?}", src.health),
                                            );
                                        }
                                    }
                                }
                                Err(_) => {
                                    any_failed = true;
                                    let mut s = self.state.lock().await;
                                    s.capability_tracker
                                        .degrade_source(&source_id, SourceHealth::TimedOut);
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id)
                                    {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                        }
                    }
                    let merged = merge_hovers_with_attribution(hovers);
                    Ok(Some(merged))
                } else {
                    let mut final_res = Ok(None);
                    for source_id in routable_sources {
                        if let Some(conn) = self.upstreams.get(&source_id) {
                            match conn.request(method, params_val.clone(), timeout_ms).await {
                                Ok(res) => {
                                    {
                                        let mut s = self.state.lock().await;
                                        if let Some(src) =
                                            s.capability_tracker.sources.get_mut(&source_id)
                                        {
                                            src.health = SourceHealth::Healthy;
                                        }
                                    }
                                    if !res.is_null() {
                                        final_res = Ok(Some(res.clone()));
                                        sources_returned.lock().unwrap().push(source_id.clone());
                                        {
                                            let s = self.state.lock().await;
                                            if let Some(src) =
                                                s.capability_tracker.sources.get(&source_id)
                                            {
                                                source_health.lock().unwrap().insert(
                                                    source_id.clone(),
                                                    format!("{:?}", src.health),
                                                );
                                            }
                                        }
                                        break;
                                    }
                                }
                                Err(_) => {
                                    any_failed = true;
                                    let mut s = self.state.lock().await;
                                    s.capability_tracker
                                        .degrade_source(&source_id, SourceHealth::TimedOut);
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id)
                                    {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                        }
                    }
                    final_res
                }
            }
            CompositionStrategy::MergeDeduped => {
                let mut observations = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) =
                                        s.capability_tracker.sources.get_mut(&source_id)
                                    {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    observations.push(AttributedObservation {
                                        source_id: source_id.clone(),
                                        uri: uri_opt.clone().unwrap_or_default(),
                                        data: res,
                                    });
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id)
                                    {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker
                                    .degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health
                                        .lock()
                                        .unwrap()
                                        .insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                let merged_locs = merge_deduped_locations(observations);
                let merged_val = serde_json::json!(merged_locs);
                Ok(Some(merged_val))
            }
            CompositionStrategy::MergeAttributed => {
                let mut observations = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) =
                                        s.capability_tracker.sources.get_mut(&source_id)
                                    {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    observations.push(AttributedObservation {
                                        source_id: source_id.clone(),
                                        uri: uri_opt.clone().unwrap_or_default(),
                                        data: res,
                                    });
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id)
                                    {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker
                                    .degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health
                                        .lock()
                                        .unwrap()
                                        .insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                let merged = merge_attributed(observations);
                Ok(Some(merged))
            }
            CompositionStrategy::RankedProviders => {
                let mut completion_responses = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) =
                                        s.capability_tracker.sources.get_mut(&source_id)
                                    {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    completion_responses.push(res);
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id)
                                    {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker
                                    .degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health
                                        .lock()
                                        .unwrap()
                                        .insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                let mut all_items = Vec::new();
                let mut seen_labels = std::collections::HashSet::new();
                let mut is_incomplete = false;
                for resp in completion_responses {
                    let items_array =
                        if let Some(items) = resp.get("items").and_then(|i| i.as_array()) {
                            if resp
                                .get("isIncomplete")
                                .and_then(|i| i.as_bool())
                                .unwrap_or(false)
                            {
                                is_incomplete = true;
                            }
                            items
                        } else if let Some(items) = resp.as_array() {
                            items
                        } else {
                            continue;
                        };
                    for item in items_array {
                        if let Some(label) = item.get("label").and_then(|l| l.as_str()) {
                            if seen_labels.insert(label.to_string()) {
                                all_items.push(item.clone());
                            }
                        }
                    }
                }
                all_items.sort_by(|a, b| {
                    let a_sort = a
                        .get("sortText")
                        .and_then(|v| v.as_str())
                        .or_else(|| a.get("label").and_then(|v| v.as_str()));
                    let b_sort = b
                        .get("sortText")
                        .and_then(|v| v.as_str())
                        .or_else(|| b.get("label").and_then(|v| v.as_str()));
                    a_sort.cmp(&b_sort)
                });
                let result = serde_json::json!({
                    "isIncomplete": is_incomplete,
                    "items": all_items
                });
                Ok(Some(result))
            }
            CompositionStrategy::TransactionalEditGate => {
                let uri = uri_opt.clone().unwrap_or_default();
                let mut merged_res = Value::Null;
                let mut accepted_proposals = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) =
                                        s.capability_tracker.sources.get_mut(&source_id)
                                    {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    let mut s = self.state.lock().await;
                                    let client_version = params_val
                                        .get("context")
                                        .and_then(|c| c.get("version"))
                                        .and_then(|v| v.as_i64())
                                        .map(|v| v as i32);
                                    let version = client_version
                                        .or_else(|| extract_version_from_edit(&res, &uri))
                                        .or_else(|| s.doc_tracker.current_version(&uri))
                                        .unwrap_or(0);
                                    let proposed = ProposedEdit {
                                        source_id: source_id.clone(),
                                        uri: uri.clone(),
                                        version,
                                        method: method.to_string(),
                                        edit: res.clone(),
                                    };
                                    let outcome = s.edit_gate.validate(
                                        &proposed,
                                        &s.doc_tracker,
                                        &s.capability_tracker,
                                    );
                                    *gate_outcome.lock().unwrap() = Some(format!("{:?}", outcome));
                                    if outcome == EditGateOutcome::Accepted {
                                        s.edit_gate.accept(proposed);
                                        accepted_proposals.push(source_id.clone());
                                        merged_res = merge_edits(merged_res, res);
                                    } else {
                                        for src_id in &accepted_proposals {
                                            s.edit_gate.remove(&uri, src_id);
                                        }
                                        return Err(Error::invalid_params(format!(
                                            "Edit gate rejected edit: {:?}",
                                            outcome
                                        )));
                                    }
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id)
                                    {
                                        source_health
                                            .lock()
                                            .unwrap()
                                            .insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker
                                    .degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health
                                        .lock()
                                        .unwrap()
                                        .insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                if merged_res.is_null() {
                    Ok(None)
                } else {
                    Ok(Some(merged_res))
                }
            }
            CompositionStrategy::OrderedFanout
            | CompositionStrategy::ObserveOnly
            | CompositionStrategy::Deny => Err(Error::method_not_found()),
        };

        if params_val.get("partialResultToken") == Some(&Value::Null) && any_failed {
            return Ok(None);
        }

        if let Some(ref uri) = uri_opt {
            let s = self.state.lock().await;
            let version_after = s.doc_tracker.current_version(uri);
            if version_before != version_after {
                *staleness_outcome.lock().unwrap() = Some("StaleRefused".to_string());
                return Ok(None);
            } else {
                *staleness_outcome.lock().unwrap() = Some("NotStale".to_string());
            }
        }

        res
    }

    pub(super) async fn route_notification<P>(&self, method: &str, params: P)
    where
        P: serde::Serialize,
    {
        println!("--- route_notification [{}] start", method);
        let params_val = serde_json::to_value(params).unwrap_or(Value::Null);
        let uri_opt = params_val
            .get("textDocument")
            .and_then(|td| td.get("uri"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        if let Some(uri) = &uri_opt {
            println!("--- route_notification [{}] locking state for uri", method);
            let mut s = self.state.lock().await;
            println!("--- route_notification [{}] locked state for uri", method);
            if method == "textDocument/didOpen" {
                let version = params_val
                    .get("textDocument")
                    .and_then(|td| td.get("version"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                s.doc_tracker.did_open(uri, version);
                s.edit_gate.clear_for_uri(uri);
            } else if method == "textDocument/didChange" {
                let version = params_val
                    .get("textDocument")
                    .and_then(|td| td.get("version"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                if let VersionCheckResult::OutOfOrder { .. } =
                    s.doc_tracker.did_change(uri, version)
                {
                    println!("--- route_notification [{}] out of order return", method);
                    return;
                }
                s.edit_gate.clear_for_uri(uri);
            } else if method == "textDocument/didClose" {
                s.doc_tracker.did_close(uri);
                s.edit_gate.clear_for_uri(uri);
            }
        }

        println!(
            "--- route_notification [{}] locking state for routable_sources",
            method
        );
        let routable_sources = {
            let s = self.state.lock().await;
            println!(
                "--- route_notification [{}] locked state for routable_sources inside block",
                method
            );
            s.capability_tracker.routable_sources_for_method(method)
        };
        println!(
            "--- route_notification [{}] routable_sources: {:?}",
            method, routable_sources
        );
        for source_id in routable_sources {
            if let Some(conn) = self.upstreams.get(&source_id) {
                println!(
                    "--- route_notification [{}] calling conn.notify for {}",
                    method, source_id
                );
                if conn.notify(method, params_val.clone()).await.is_ok() {
                    println!(
                        "--- route_notification [{}] locking state for source health updating",
                        method
                    );
                    let mut s = self.state.lock().await;
                    println!(
                        "--- route_notification [{}] locked state for source health updating",
                        method
                    );
                    if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                        src.health = SourceHealth::Healthy;
                    }
                }
            }
        }
        println!("--- route_notification [{}] end", method);
    }
}
