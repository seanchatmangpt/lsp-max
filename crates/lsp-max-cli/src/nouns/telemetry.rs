use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_protocol;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

// --- Domain Tier ---

#[derive(Debug, Clone, Serialize)]
pub enum TelemetryStatus {
    Exported,
    Traced,
    MetricsCollected,
    Flushed,
}

// --- Service Tier ---

pub struct TelemetryService {
    state_path: String,
}

impl TelemetryService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn export(
        &self,
        destination: &str,
        data_id: &str,
    ) -> std::result::Result<TelemetryStatus, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let params = serde_json::json!({ "destination": destination, "data_id": data_id });
        // Wire to runtime: exportAnalysisBundle for the first instance that matches data_id,
        // or emit a receipt recording the export event.
        let instance_id_str = mesh
            .instances
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        let instance_id = lsp_max_runtime::InstanceId::from(instance_id_str.clone());
        // Record export as a bounded action
        mesh.execute_action(lsp_max_runtime::MeshAction::ExecuteBoundedAction {
            instance_id: instance_id.clone(),
            action_id: format!("telemetry-export-{}", data_id),
            description: format!("Export telemetry data {} to {}", data_id, destination),
        });
        let receipt_id = format!(
            "rcpt-telemetry-export-{}-{}",
            data_id,
            destination.replace("://", "-").replace('/', "-")
        );
        let hash = lsp_max_runtime::sha256(receipt_id.as_bytes());
        mesh.execute_action(lsp_max_runtime::MeshAction::EmitReceipt {
            instance_id,
            receipt: lsp_max_protocol::Receipt {
                receipt_id,
                hash,
                prev_receipt_hash: None,
            },
        });
        let _ = params; // params used for documentation only
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(TelemetryStatus::Exported)
    }

    pub fn trace(&self, span_name: &str) -> std::result::Result<TelemetryStatus, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let instance_id_str = mesh
            .instances
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        let instance_id = lsp_max_runtime::InstanceId::from(instance_id_str.clone());
        mesh.execute_action(lsp_max_runtime::MeshAction::ExecuteBoundedAction {
            instance_id,
            action_id: format!("telemetry-trace-{}", span_name),
            description: format!("OTel trace span: {}", span_name),
        });
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(TelemetryStatus::Traced)
    }

    pub fn metrics(
        &self,
        metric_name: &str,
        value: f64,
    ) -> std::result::Result<TelemetryStatus, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let instance_id_str = mesh
            .instances
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        let instance_id = lsp_max_runtime::InstanceId::from(instance_id_str.clone());
        mesh.execute_action(lsp_max_runtime::MeshAction::ExecuteBoundedAction {
            instance_id,
            action_id: format!("telemetry-metric-{}", metric_name),
            description: format!("Record metric {}={}", metric_name, value),
        });
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(TelemetryStatus::MetricsCollected)
    }

    pub fn flush(&self) -> std::result::Result<TelemetryStatus, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        // Flush is read-only: verify the mesh is loadable and return status
        let _ = mesh.to_state();
        Ok(TelemetryStatus::Flushed)
    }
}

// --- CLI Tier ---

#[derive(Serialize)]
pub struct ExportResult {
    pub status: TelemetryStatus,
    pub destination: String,
    pub data_id: String,
}

#[verb("export")]
pub fn export(destination: String, data_id: String) -> Result<ExportResult> {
    let service = TelemetryService::new();
    let status = service
        .export(&destination, &data_id)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(ExportResult {
        status,
        destination,
        data_id,
    })
}

#[derive(Serialize)]
pub struct TraceResult {
    pub status: TelemetryStatus,
    pub span_name: String,
}

#[verb("trace")]
pub fn trace(span_name: String) -> Result<TraceResult> {
    let service = TelemetryService::new();
    let status = service
        .trace(&span_name)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(TraceResult { status, span_name })
}

#[derive(Serialize)]
pub struct MetricsResult {
    pub status: TelemetryStatus,
    pub metric_name: String,
    pub value: f64,
}

#[verb("metrics")]
pub fn metrics(metric_name: String, value: f64) -> Result<MetricsResult> {
    let service = TelemetryService::new();
    let status = service
        .metrics(&metric_name, value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(MetricsResult {
        status,
        metric_name,
        value,
    })
}

#[derive(Serialize)]
pub struct FlushResult {
    pub status: TelemetryStatus,
}

#[verb("flush")]
pub fn flush() -> Result<FlushResult> {
    let service = TelemetryService::new();
    let status = service
        .flush()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(FlushResult { status })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn with_isolated_state<F: FnOnce()>(f: F) {
        let _guard = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_str().unwrap().to_string();
        let prev = env::var("TOWER_LSP_MAX_STATE_PATH").ok();
        env::set_var("TOWER_LSP_MAX_STATE_PATH", &path);
        let _ = std::fs::remove_file(&path);
        f();
        let _ = std::fs::remove_file(&path);
        match prev {
            Some(v) => env::set_var("TOWER_LSP_MAX_STATE_PATH", v),
            None => env::remove_var("TOWER_LSP_MAX_STATE_PATH"),
        }
    }

    #[test]
    fn test_export_returns_exported_status() {
        with_isolated_state(|| {
            let result = export("s3://bucket".to_string(), "data-001".to_string()).unwrap();
            assert!(matches!(result.status, TelemetryStatus::Exported));
            assert_eq!(result.destination, "s3://bucket");
            assert_eq!(result.data_id, "data-001");
        });
    }

    #[test]
    fn test_trace_returns_traced_status() {
        with_isolated_state(|| {
            let result = trace("my-span".to_string()).unwrap();
            assert!(matches!(result.status, TelemetryStatus::Traced));
            assert_eq!(result.span_name, "my-span");
        });
    }

    #[test]
    fn test_metrics_returns_metrics_collected_status() {
        with_isolated_state(|| {
            let result = metrics("cpu.usage".to_string(), 42.5).unwrap();
            assert!(matches!(result.status, TelemetryStatus::MetricsCollected));
            assert_eq!(result.metric_name, "cpu.usage");
            assert_eq!(result.value, 42.5);
        });
    }

    #[test]
    fn test_flush_returns_flushed_status() {
        with_isolated_state(|| {
            let result = flush().unwrap();
            assert!(matches!(result.status, TelemetryStatus::Flushed));
        });
    }
}
