use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEntry {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricTrend {
    pub name: String,
    pub current: f64,
    pub baseline: f64,
    pub delta: f64,
    pub trend: String,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct MetricsService {
    state_path: String,
}

impl MetricsService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn collect(&self) -> std::result::Result<Vec<MetricEntry>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path)
            .unwrap_or_else(|_| AutonomicMesh::new());

        let instance_count = mesh.instances.len() as f64;

        let mut diag_total: f64 = 0.0;
        let mut diag_errors: f64 = 0.0;
        let mut diag_warnings: f64 = 0.0;
        let mut score_sum: f64 = 0.0;
        let mut receipt_total: f64 = 0.0;

        for inst in mesh.instances.values() {
            let diags = &inst.diagnostics;
            diag_total += diags.len() as f64;
            for d in diags {
                match d.lsp.severity {
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::ERROR => {
                        diag_errors += 1.0;
                    }
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::WARNING => {
                        diag_warnings += 1.0;
                    }
                    _ => {}
                }
            }
            score_sum += inst.conformance_score();
            receipt_total += inst.receipts.len() as f64;
        }

        let avg_score = if instance_count > 0.0 {
            score_sum / instance_count
        } else {
            0.0
        };

        // Repair plan counts are tracked through bounded action names recorded
        // in `executed_bounded_actions`. Actions carrying status "OPEN" or
        // "ADMITTED" in their action_id are counted accordingly.
        let repairs_open = mesh
            .executed_bounded_actions
            .iter()
            .filter(|a| a.contains("repair-") && a.contains("OPEN"))
            .count() as f64;

        let repairs_admitted = mesh
            .executed_bounded_actions
            .iter()
            .filter(|a| a.contains("repair-") && a.contains("ADMITTED"))
            .count() as f64;

        let metrics = vec![
            MetricEntry {
                name: "lsp_max.instances.total".into(),
                value: instance_count,
                unit: "count".into(),
                labels: HashMap::new(),
            },
            MetricEntry {
                name: "lsp_max.diagnostics.total".into(),
                value: diag_total,
                unit: "count".into(),
                labels: HashMap::new(),
            },
            MetricEntry {
                name: "lsp_max.diagnostics.errors".into(),
                value: diag_errors,
                unit: "count".into(),
                labels: HashMap::new(),
            },
            MetricEntry {
                name: "lsp_max.diagnostics.warnings".into(),
                value: diag_warnings,
                unit: "count".into(),
                labels: HashMap::new(),
            },
            MetricEntry {
                name: "lsp_max.conformance.avg_score".into(),
                value: avg_score,
                unit: "score".into(),
                labels: HashMap::new(),
            },
            MetricEntry {
                name: "lsp_max.receipts.total".into(),
                value: receipt_total,
                unit: "count".into(),
                labels: HashMap::new(),
            },
            MetricEntry {
                name: "lsp_max.repairs.open".into(),
                value: repairs_open,
                unit: "count".into(),
                labels: HashMap::new(),
            },
            MetricEntry {
                name: "lsp_max.repairs.admitted".into(),
                value: repairs_admitted,
                unit: "count".into(),
                labels: HashMap::new(),
            },
        ];

        Ok(metrics)
    }

    pub fn format_prometheus(metrics: &[MetricEntry]) -> String {
        let mut out = String::new();
        for m in metrics {
            let prom_name = m.name.replace('.', "_").replace('-', "_");
            out.push_str(&format!("# TYPE {} gauge\n", prom_name));
            let label_str = if m.labels.is_empty() {
                String::new()
            } else {
                let pairs: Vec<String> = m
                    .labels
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", pairs.join(","))
            };
            out.push_str(&format!("{}{} {}\n", prom_name, label_str, m.value));
        }
        out
    }

    pub fn format_csv(metrics: &[MetricEntry]) -> String {
        let mut out = String::from("name,value,unit,labels\n");
        for m in metrics {
            let labels_str = m
                .labels
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(";");
            out.push_str(&format!(
                "{},{},{},{}\n",
                m.name, m.value, m.unit, labels_str
            ));
        }
        out
    }

    pub fn baseline_path() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{}/.lsp-max-metrics-baseline.json", home)
    }

    pub fn load_baseline() -> Option<Vec<MetricEntry>> {
        let path = Self::baseline_path();
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save_baseline(metrics: &[MetricEntry]) -> std::result::Result<(), String> {
        let path = Self::baseline_path();
        let json = serde_json::to_string_pretty(metrics).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Serialize)]
pub struct MetricsListResult {
    pub metrics: Vec<MetricEntry>,
    pub snapshot_at: String,
}

#[verb("list")]
pub fn list() -> Result<MetricsListResult> {
    let svc = MetricsService::new();
    let metrics = svc.collect().map_err(NounVerbError::execution_error)?;
    let snapshot_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into());
    Ok(MetricsListResult {
        metrics,
        snapshot_at,
    })
}

#[derive(Serialize)]
pub struct MetricsExportResult {
    pub format: String,
    pub metric_count: usize,
    pub output: String,
}

#[verb("export")]
pub fn export(format: Option<String>) -> Result<MetricsExportResult> {
    let svc = MetricsService::new();
    let metrics = svc.collect().map_err(NounVerbError::execution_error)?;
    let metric_count = metrics.len();
    let fmt = format.as_deref().unwrap_or("json");

    let output = match fmt {
        "prometheus" => MetricsService::format_prometheus(&metrics),
        "csv" => MetricsService::format_csv(&metrics),
        _ => serde_json::to_string_pretty(&metrics)
            .map_err(|e| NounVerbError::execution_error(e.to_string()))?,
    };

    Ok(MetricsExportResult {
        format: fmt.to_string(),
        metric_count,
        output,
    })
}

#[derive(Serialize)]
pub struct MetricsTrendingResult {
    pub comparisons: Vec<MetricTrend>,
    pub status: String,
}

#[verb("trending")]
pub fn trending() -> Result<MetricsTrendingResult> {
    let svc = MetricsService::new();
    let current = svc.collect().map_err(NounVerbError::execution_error)?;

    let baseline_opt = MetricsService::load_baseline();

    if baseline_opt.is_none() {
        let comparisons = current
            .iter()
            .map(|m| MetricTrend {
                name: m.name.clone(),
                current: m.value,
                baseline: 0.0,
                delta: 0.0,
                trend: "UNKNOWN".into(),
            })
            .collect();
        return Ok(MetricsTrendingResult {
            comparisons,
            status: "UNKNOWN".into(),
        });
    }

    let baseline = baseline_opt.unwrap();
    let baseline_map: HashMap<String, f64> = baseline
        .iter()
        .map(|m| (m.name.clone(), m.value))
        .collect();

    let mut degrading = 0usize;
    let mut comparisons = Vec::new();

    for m in &current {
        let base_val = baseline_map.get(&m.name).copied().unwrap_or(0.0);
        let delta = m.value - base_val;

        // For diagnostic/error/warning counts and open repairs: rising = degrading.
        // For conformance score: falling = degrading.
        let trend = if delta.abs() < f64::EPSILON {
            "STABLE"
        } else if m.name.contains("conformance") {
            if delta < 0.0 {
                degrading += 1;
                "DEGRADING"
            } else {
                "IMPROVING"
            }
        } else if m.name.contains("diagnostics") || m.name.contains("repairs.open") {
            if delta > 0.0 {
                degrading += 1;
                "DEGRADING"
            } else {
                "IMPROVING"
            }
        } else {
            "STABLE"
        };

        comparisons.push(MetricTrend {
            name: m.name.clone(),
            current: m.value,
            baseline: base_val,
            delta,
            trend: trend.into(),
        });
    }

    let total = comparisons.len();
    let status = if degrading == 0 {
        "ADMITTED"
    } else if degrading * 2 >= total {
        "BLOCKED"
    } else {
        "PARTIAL"
    };

    Ok(MetricsTrendingResult {
        comparisons,
        status: status.into(),
    })
}

#[derive(Serialize)]
pub struct MetricsBaselineResult {
    pub saved_at: String,
    pub metric_count: usize,
    pub path: String,
}

#[verb("baseline")]
pub fn baseline() -> Result<MetricsBaselineResult> {
    let svc = MetricsService::new();
    let metrics = svc.collect().map_err(NounVerbError::execution_error)?;
    let metric_count = metrics.len();
    MetricsService::save_baseline(&metrics).map_err(NounVerbError::execution_error)?;
    let path = MetricsService::baseline_path();
    let saved_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into());
    Ok(MetricsBaselineResult {
        saved_at,
        metric_count,
        path,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

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
        // SAFETY: test-only, guarded by TEST_ENV_LOCK
        unsafe {
            env::set_var("LSP_MAX_STATE_PATH", &path);
        }
        let _ = std::fs::remove_file(&path);
        f();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn collect_returns_eight_metric_entries() {
        with_isolated_state(|| {
            let svc = MetricsService::new();
            let result = svc.collect().unwrap();
            assert_eq!(result.len(), 8);
        });
    }

    #[test]
    fn format_prometheus_contains_type_comment() {
        let m = MetricEntry {
            name: "lsp_max.instances.total".into(),
            value: 3.0,
            unit: "count".into(),
            labels: HashMap::new(),
        };
        let out = MetricsService::format_prometheus(&[m]);
        assert!(out.contains("# TYPE lsp_max_instances_total gauge"));
        assert!(out.contains("lsp_max_instances_total 3"));
    }

    #[test]
    fn format_csv_has_header_row() {
        let m = MetricEntry {
            name: "lsp_max.diagnostics.total".into(),
            value: 0.0,
            unit: "count".into(),
            labels: HashMap::new(),
        };
        let out = MetricsService::format_csv(&[m]);
        assert!(out.starts_with("name,value,unit,labels\n"));
        assert!(out.contains("lsp_max.diagnostics.total"));
    }

    #[test]
    fn trending_returns_unknown_when_no_baseline() {
        with_isolated_state(|| {
            // No baseline file present; all statuses must be UNKNOWN
            let result = trending().unwrap();
            assert_eq!(result.status, "UNKNOWN");
            for c in &result.comparisons {
                assert_eq!(c.trend, "UNKNOWN");
            }
        });
    }
}
