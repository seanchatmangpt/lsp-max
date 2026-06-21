use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

// ==============================================================================
// 1. Service Tier
// ==============================================================================

pub struct ExportService {
    state_path: String,
}

impl ExportService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Serialize the full mesh state.  Returns (json_string, instance_count).
    pub fn state_json(&self, pretty: bool) -> std::result::Result<(String, usize), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let instance_count = mesh.instances.len();
        let s = if pretty {
            serde_json::to_string_pretty(&mesh.to_state()).map_err(|e| e.to_string())?
        } else {
            serde_json::to_string(&mesh.to_state()).map_err(|e| e.to_string())?
        };
        Ok((s, instance_count))
    }

    /// Write bytes to a file; returns bytes written.
    pub fn write_to_file(path: &str, content: &str) -> std::result::Result<usize, String> {
        std::fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(content.len())
    }

    /// Collect all diagnostics across instances as CSV rows.
    /// Columns: instance_id,severity,code,message
    pub fn diagnostics_csv(&self) -> std::result::Result<(String, usize), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let mut rows = vec!["instance_id,severity,code,message".to_string()];
        let mut count = 0;
        for (id, inst) in &mesh.instances {
            for diag in &inst.diagnostics {
                let sev = match diag.lsp.severity {
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::ERROR => "ERROR",
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::WARNING => "WARNING",
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::INFORMATION => "INFO",
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::HINT => "HINT",
                    _ => "UNKNOWN",
                };
                let code = match &diag.lsp.code {
                    Some(lsp_types_max::NumberOrString::String(s)) => s.clone(),
                    Some(lsp_types_max::NumberOrString::Number(n)) => n.to_string(),
                    None => String::new(),
                };
                // Escape message: replace commas and newlines so CSV stays parsable.
                let message = diag
                    .lsp
                    .message
                    .replace('"', "\"\"")
                    .replace('\n', " ")
                    .replace('\r', "");
                rows.push(format!("{},{},{},\"{}\"", id, sev, code, message));
                count += 1;
            }
        }
        Ok((rows.join("\n"), count))
    }

    /// Collect all diagnostics across instances as a JSON array.
    pub fn diagnostics_json(&self) -> std::result::Result<(String, usize), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let mut entries: Vec<serde_json::Value> = Vec::new();
        for (id, inst) in &mesh.instances {
            for diag in &inst.diagnostics {
                let mut v = serde_json::to_value(diag).map_err(|e| e.to_string())?;
                if let Some(obj) = v.as_object_mut() {
                    obj.insert(
                        "instance_id".to_string(),
                        serde_json::Value::String(id.clone()),
                    );
                }
                entries.push(v);
            }
        }
        let count = entries.len();
        let s = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
        Ok((s, count))
    }

    /// Collect all receipts across instances as a JSON array.
    pub fn receipts_json(&self) -> std::result::Result<(String, usize), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let mut all_receipts: Vec<serde_json::Value> = Vec::new();
        for (id, inst) in &mesh.instances {
            for r in &inst.receipts {
                let mut v = serde_json::to_value(r).map_err(|e| e.to_string())?;
                if let Some(obj) = v.as_object_mut() {
                    obj.insert(
                        "instance_id".to_string(),
                        serde_json::Value::String(id.clone()),
                    );
                }
                all_receipts.push(v);
            }
        }
        let receipt_count = all_receipts.len();
        let s = serde_json::to_string_pretty(&all_receipts).map_err(|e| e.to_string())?;
        Ok((s, receipt_count))
    }

    /// Build conformance rows per instance.
    /// Returns (content, instance_count) in either CSV or JSON.
    pub fn conformance(&self, format: &str) -> std::result::Result<(String, usize), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let instance_count = mesh.instances.len();

        if format == "csv" {
            let mut rows =
                vec!["instance_id,score,admitted_count,refused_count,unknown_count".to_string()];
            for (id, inst) in &mesh.instances {
                let score = inst.conformance_score();
                let cv = lsp_max_runtime::build_conformance_vector(&inst.diagnostics);
                rows.push(format!(
                    "{},{},{},{},{}",
                    id,
                    score,
                    cv.admitted.len(),
                    cv.refused.len(),
                    cv.unknown.len()
                ));
            }
            Ok((rows.join("\n"), instance_count))
        } else {
            let entries: Vec<serde_json::Value> = mesh
                .instances
                .iter()
                .map(|(id, inst)| {
                    let score = inst.conformance_score();
                    let cv = lsp_max_runtime::build_conformance_vector(&inst.diagnostics);
                    serde_json::json!({
                        "instance_id": id,
                        "score": score,
                        "admitted_count": cv.admitted.len(),
                        "refused_count": cv.refused.len(),
                        "unknown_count": cv.unknown.len(),
                    })
                })
                .collect();
            let s = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
            Ok((s, instance_count))
        }
    }
}

impl Default for ExportService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ExportStateResult {
    /// File path written, or "<inline>" when dest was None.
    pub dest: String,
    /// Bytes written (0 for inline).
    pub bytes_written: usize,
    pub instance_count: usize,
    pub status: String,
    /// Populated when dest is None; empty string otherwise.
    pub output: String,
}

#[verb("state")]
pub fn state(dest: Option<String>, format: Option<String>) -> Result<ExportStateResult> {
    let svc = ExportService::new();
    let pretty = format.as_deref().unwrap_or("json") == "pretty";
    let (json, instance_count) = svc
        .state_json(pretty)
        .map_err(NounVerbError::execution_error)?;

    match dest {
        Some(path) => {
            let bytes_written = ExportService::write_to_file(&path, &json)
                .map_err(NounVerbError::execution_error)?;
            Ok(ExportStateResult {
                dest: path,
                bytes_written,
                instance_count,
                status: "ADMITTED".to_string(),
                output: String::new(),
            })
        }
        None => Ok(ExportStateResult {
            dest: "<inline>".to_string(),
            bytes_written: json.len(),
            instance_count,
            status: "ADMITTED".to_string(),
            output: json,
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportDiagnosticsResult {
    /// File path written, or "<inline>" when dest was None.
    pub dest: String,
    pub count: usize,
    pub format: String,
    pub status: String,
    /// Populated when dest is None; empty string otherwise.
    pub output: String,
}

#[verb("diagnostics")]
pub fn diagnostics(
    dest: Option<String>,
    format: Option<String>,
) -> Result<ExportDiagnosticsResult> {
    let svc = ExportService::new();
    let fmt = format.unwrap_or_else(|| "json".to_string());

    let (content, count) = if fmt == "csv" {
        svc.diagnostics_csv()
            .map_err(NounVerbError::execution_error)?
    } else {
        svc.diagnostics_json()
            .map_err(NounVerbError::execution_error)?
    };

    match dest {
        Some(path) => {
            ExportService::write_to_file(&path, &content)
                .map_err(NounVerbError::execution_error)?;
            Ok(ExportDiagnosticsResult {
                dest: path,
                count,
                format: fmt,
                status: "ADMITTED".to_string(),
                output: String::new(),
            })
        }
        None => Ok(ExportDiagnosticsResult {
            dest: "<inline>".to_string(),
            count,
            format: fmt,
            status: "ADMITTED".to_string(),
            output: content,
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportReceiptsResult {
    /// File path written, or "<inline>" when dest was None.
    pub dest: String,
    pub receipt_count: usize,
    pub status: String,
    /// Populated when dest is None; empty string otherwise.
    pub output: String,
}

#[verb("receipts")]
pub fn receipts(dest: Option<String>) -> Result<ExportReceiptsResult> {
    let svc = ExportService::new();
    let (content, receipt_count) = svc
        .receipts_json()
        .map_err(NounVerbError::execution_error)?;

    match dest {
        Some(path) => {
            ExportService::write_to_file(&path, &content)
                .map_err(NounVerbError::execution_error)?;
            Ok(ExportReceiptsResult {
                dest: path,
                receipt_count,
                status: "ADMITTED".to_string(),
                output: String::new(),
            })
        }
        None => Ok(ExportReceiptsResult {
            dest: "<inline>".to_string(),
            receipt_count,
            status: "ADMITTED".to_string(),
            output: content,
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportConformanceResult {
    /// File path written, or "<inline>" when dest was None.
    pub dest: String,
    pub instance_count: usize,
    pub format: String,
    pub status: String,
    /// Populated when dest is None; empty string otherwise.
    pub output: String,
}

#[verb("conformance")]
pub fn conformance(
    dest: Option<String>,
    format: Option<String>,
) -> Result<ExportConformanceResult> {
    let svc = ExportService::new();
    let fmt = format.unwrap_or_else(|| "json".to_string());
    let (content, instance_count) = svc
        .conformance(&fmt)
        .map_err(NounVerbError::execution_error)?;

    match dest {
        Some(path) => {
            ExportService::write_to_file(&path, &content)
                .map_err(NounVerbError::execution_error)?;
            Ok(ExportConformanceResult {
                dest: path,
                instance_count,
                format: fmt,
                status: "ADMITTED".to_string(),
                output: String::new(),
            })
        }
        None => Ok(ExportConformanceResult {
            dest: "<inline>".to_string(),
            instance_count,
            format: fmt,
            status: "ADMITTED".to_string(),
            output: content,
        }),
    }
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_mesh_with_instance() -> (tempfile::NamedTempFile, ExportService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-a"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = ExportService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    fn make_empty_mesh() -> (tempfile::NamedTempFile, ExportService) {
        let mesh = AutonomicMesh::new();
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = ExportService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // --- state_json ---

    #[test]
    fn state_json_returns_ok_for_valid_mesh() {
        let (_f, svc) = make_temp_mesh_with_instance();
        assert!(svc.state_json(false).is_ok());
        assert!(svc.state_json(true).is_ok());
    }

    #[test]
    fn state_json_output_contains_instances_key() {
        let (_f, svc) = make_temp_mesh_with_instance();
        let (json, count) = svc.state_json(true).unwrap();
        assert_eq!(count, 1, "one instance must be reported");
        assert!(json.contains("instances"), "serialised state must have 'instances' key");
    }

    #[test]
    fn state_json_fails_on_missing_file() {
        let svc = ExportService {
            state_path: "/tmp/nonexistent-export-test.json".to_string(),
        };
        assert!(svc.state_json(false).is_err());
    }

    // --- diagnostics ---

    #[test]
    fn diagnostics_csv_returns_ok_and_has_header() {
        let (_f, svc) = make_temp_mesh_with_instance();
        let (csv, _count) = svc.diagnostics_csv().unwrap();
        assert!(
            csv.starts_with("instance_id,severity,code,message"),
            "CSV must begin with the header row"
        );
    }

    #[test]
    fn diagnostics_csv_empty_mesh_has_header_only() {
        let (_f, svc) = make_empty_mesh();
        let (csv, count) = svc.diagnostics_csv().unwrap();
        assert_eq!(count, 0);
        assert_eq!(csv.trim(), "instance_id,severity,code,message");
    }

    #[test]
    fn diagnostics_json_returns_ok() {
        let (_f, svc) = make_temp_mesh_with_instance();
        assert!(svc.diagnostics_json().is_ok());
    }

    #[test]
    fn diagnostics_json_output_is_valid_json_array() {
        let (_f, svc) = make_temp_mesh_with_instance();
        let (json, _) = svc.diagnostics_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("must parse as JSON");
        assert!(parsed.is_array(), "diagnostics export must be a JSON array");
    }

    // --- receipts ---

    #[test]
    fn receipts_json_returns_ok() {
        let (_f, svc) = make_temp_mesh_with_instance();
        assert!(svc.receipts_json().is_ok());
    }

    #[test]
    fn receipts_json_empty_instance_has_zero_receipts() {
        let (_f, svc) = make_temp_mesh_with_instance();
        let (_json, receipt_count) = svc.receipts_json().unwrap();
        assert_eq!(receipt_count, 0);
    }

    // --- conformance ---

    #[test]
    fn conformance_json_format_returns_ok() {
        let (_f, svc) = make_temp_mesh_with_instance();
        assert!(svc.conformance("json").is_ok());
    }

    #[test]
    fn conformance_csv_format_returns_ok() {
        let (_f, svc) = make_temp_mesh_with_instance();
        let (csv, count) = svc.conformance("csv").unwrap();
        assert_eq!(count, 1);
        assert!(
            csv.starts_with("instance_id,score"),
            "CSV conformance must start with header"
        );
    }

    #[test]
    fn conformance_json_is_parseable_array() {
        let (_f, svc) = make_temp_mesh_with_instance();
        let (json, _) = svc.conformance("json").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("must parse as JSON");
        assert!(parsed.is_array());
    }

    // --- write_to_file ---

    #[test]
    fn write_to_file_returns_byte_count() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let content = "hello world";
        let bytes = ExportService::write_to_file(f.path().to_str().unwrap(), content).unwrap();
        assert_eq!(bytes, content.len());
    }

    #[test]
    fn write_to_file_fails_for_invalid_path() {
        assert!(ExportService::write_to_file("/no/such/dir/file.json", "x").is_err());
    }
}
