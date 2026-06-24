use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An object in the OCEL 2.0 sense — one per LspInstance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelObject {
    #[serde(rename = "ocel:type")]
    pub object_type: String,
    #[serde(rename = "ocel:ovmap")]
    pub attributes: serde_json::Value,
}

/// A single event in the OCEL 2.0 log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelEvent {
    #[serde(rename = "ocel:activity")]
    pub activity: String,
    #[serde(rename = "ocel:timestamp")]
    pub timestamp: String,
    /// Instance IDs involved in this event.
    #[serde(rename = "ocel:omap")]
    pub object_map: Vec<String>,
    #[serde(rename = "ocel:vmap")]
    pub value_map: serde_json::Value,
}

/// Top-level global metadata block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelGlobalLog {
    #[serde(rename = "ocel:attribute-names")]
    pub attribute_names: Vec<String>,
    #[serde(rename = "ocel:object-types")]
    pub object_types: Vec<String>,
}

/// The full OCEL 2.0 document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelLog {
    #[serde(rename = "ocel:global-log")]
    pub global_log: OcelGlobalLog,
    #[serde(rename = "ocel:events")]
    pub events: HashMap<String, OcelEvent>,
    #[serde(rename = "ocel:objects")]
    pub objects: HashMap<String, OcelObject>,
}

/// Per-object case summary.
#[derive(Debug, Clone, Serialize)]
pub struct OcelCase {
    pub case_id: String,
    pub object_type: String,
    pub event_count: usize,
    pub activities: Vec<String>,
}

/// Object-type instance count.
#[derive(Debug, Clone, Serialize)]
pub struct OcelObjectTypeSummary {
    pub object_type: String,
    pub instance_count: usize,
}

/// A directly-follows arc in the Object-Centric DFG.
#[derive(Debug, Clone, Serialize)]
pub struct OcelDiscoveryArc {
    pub from_activity: String,
    pub to_activity: String,
    pub frequency: usize,
    pub object_type: String,
}

/// OC-DFG process discovery report.
#[derive(Debug, Clone, Serialize)]
pub struct OcelDiscoveryReport {
    pub object_types: Vec<String>,
    pub activities: Vec<String>,
    pub arcs: Vec<OcelDiscoveryArc>,
    pub total_events: usize,
    pub total_objects: usize,
}

pub struct OcelService {
    state_path: String,
}

impl OcelService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Build an OCEL 2.0 log from the current mesh state.
    ///
    /// Each `LspInstance` becomes one OCEL object; each entry in the instance's
    /// `event_log` becomes one OCEL event.  The activity name is the outer key of
    /// the externally-tagged `HookEvent` enum variant.
    pub fn export(&self) -> std::result::Result<OcelLog, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let mut objects: HashMap<String, OcelObject> = HashMap::new();
        let mut events: HashMap<String, OcelEvent> = HashMap::new();
        let mut event_seq = 0usize;

        for (instance_id, inst) in &mesh.instances {
            objects.insert(
                instance_id.clone(),
                OcelObject {
                    object_type: "LspInstance".to_string(),
                    attributes: serde_json::json!({
                        "conformance_score": inst.conformance_score(),
                        "diagnostic_count": inst.diagnostics.len(),
                    }),
                },
            );
        }

        // mesh.event_log is the shared event journal; each HookEvent variant carries
        // the instance_id it belongs to as a named field inside the variant payload.
        for (idx, hook_event) in mesh.event_log.iter().enumerate() {
            let raw = serde_json::to_value(hook_event).map_err(|e| e.to_string())?;
            // Externally-tagged enum: outer key = variant name = activity.
            let obj = raw.as_object();
            let activity = obj
                .and_then(|m| m.keys().next())
                .map(|s| s.to_owned())
                .unwrap_or_else(|| "Unknown".to_string());
            // The payload under the variant key carries `instance_id`.
            let event_instance_id = obj
                .and_then(|m| m.values().next())
                .and_then(|v| v.get("instance_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            event_seq += 1;
            let event_id = format!("e{event_seq}-{idx}");
            events.insert(
                event_id,
                OcelEvent {
                    activity,
                    // Wall-clock not stored on events; use a stable placeholder.
                    timestamp: "1970-01-01T00:00:00Z".to_string(),
                    object_map: vec![event_instance_id],
                    value_map: serde_json::json!({}),
                },
            );
        }

        Ok(OcelLog {
            global_log: OcelGlobalLog {
                attribute_names: vec![
                    "conformance_score".to_string(),
                    "diagnostic_count".to_string(),
                ],
                object_types: vec!["LspInstance".to_string()],
            },
            events,
            objects,
        })
    }

    /// Return per-object cases (grouped event sequences).
    ///
    /// If `instance_id` is given only events whose `ocel:omap` includes that id
    /// are counted.  Returns an empty list when there are no events — an object
    /// with no events has no traceable case.
    pub fn cases(
        &self,
        instance_id: Option<&str>,
    ) -> std::result::Result<Vec<OcelCase>, String> {
        let log = self.export()?;
        let mut case_map: HashMap<String, (String, Vec<String>)> = HashMap::new();

        for event in log.events.values() {
            for obj_id in &event.object_map {
                if let Some(filter) = instance_id {
                    if obj_id.as_str() != filter {
                        continue;
                    }
                }
                let obj_type = log
                    .objects
                    .get(obj_id)
                    .map(|o| o.object_type.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                let entry = case_map
                    .entry(obj_id.clone())
                    .or_insert_with(|| (obj_type, Vec::new()));
                entry.1.push(event.activity.clone());
            }
        }

        let mut cases: Vec<OcelCase> = case_map
            .into_iter()
            .map(|(case_id, (object_type, activities))| OcelCase {
                event_count: activities.len(),
                activities,
                case_id,
                object_type,
            })
            .collect();
        cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
        Ok(cases)
    }

    /// Return distinct object types and their instance counts.
    pub fn object_types(&self) -> std::result::Result<Vec<OcelObjectTypeSummary>, String> {
        let log = self.export()?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for obj in log.objects.values() {
            *counts.entry(obj.object_type.clone()).or_default() += 1;
        }
        let mut result: Vec<OcelObjectTypeSummary> = counts
            .into_iter()
            .map(|(object_type, instance_count)| OcelObjectTypeSummary {
                object_type,
                instance_count,
            })
            .collect();
        result.sort_by(|a, b| a.object_type.cmp(&b.object_type));
        Ok(result)
    }

    /// Object-Centric DFG discovery.
    ///
    /// For each object, the ordered sequence of activities defines the per-object
    /// trace.  Directly-follows pairs are counted per object-type to form the
    /// OC-DFG arcs.
    pub fn discover(&self) -> std::result::Result<OcelDiscoveryReport, String> {
        let log = self.export()?;

        // Per-object ordered activity sequence (insertion order = event order).
        let mut obj_sequences: HashMap<String, Vec<String>> = HashMap::new();
        for event in log.events.values() {
            for obj_id in &event.object_map {
                obj_sequences
                    .entry(obj_id.clone())
                    .or_default()
                    .push(event.activity.clone());
            }
        }

        // OC-DFG arcs: (from_activity, to_activity, object_type) → frequency.
        let mut arc_counts: HashMap<(String, String, String), usize> = HashMap::new();
        for (obj_id, activities) in &obj_sequences {
            let obj_type = log
                .objects
                .get(obj_id)
                .map(|o| o.object_type.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            for pair in activities.windows(2) {
                *arc_counts
                    .entry((pair[0].clone(), pair[1].clone(), obj_type.clone()))
                    .or_default() += 1;
            }
        }

        let mut arcs: Vec<OcelDiscoveryArc> = arc_counts
            .into_iter()
            .map(|((from, to, otype), frequency)| OcelDiscoveryArc {
                from_activity: from,
                to_activity: to,
                frequency,
                object_type: otype,
            })
            .collect();
        arcs.sort_by(|a, b| b.frequency.cmp(&a.frequency));

        let activity_set: std::collections::HashSet<String> =
            log.events.values().map(|e| e.activity.clone()).collect();
        let mut activities: Vec<String> = activity_set.into_iter().collect();
        activities.sort();

        let mut object_types = log.global_log.object_types.clone();
        object_types.sort();

        Ok(OcelDiscoveryReport {
            object_types,
            activities,
            arcs,
            total_events: log.events.len(),
            total_objects: log.objects.len(),
        })
    }
}

impl Default for OcelService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
pub struct OcelExportResult {
    pub object_count: usize,
    pub event_count: usize,
    pub object_types: Vec<String>,
    pub log: OcelLog,
}

/// Export the mesh as an OCEL 2.0 Object-Centric Event Log.
#[verb("export")]
pub fn export() -> Result<OcelExportResult> {
    let svc = OcelService::new();
    let log = svc.export().map_err(NounVerbError::execution_error)?;
    let object_count = log.objects.len();
    let event_count = log.events.len();
    let object_types = log.global_log.object_types.clone();
    Ok(OcelExportResult {
        object_count,
        event_count,
        object_types,
        log,
    })
}

#[derive(Serialize)]
pub struct OcelCasesResult {
    pub cases: Vec<OcelCase>,
    pub total: usize,
}

/// List per-object cases (grouped event sequences) from the OCEL log.
#[verb("cases")]
pub fn cases(instance_id: Option<String>) -> Result<OcelCasesResult> {
    let svc = OcelService::new();
    let cases = svc
        .cases(instance_id.as_deref())
        .map_err(NounVerbError::execution_error)?;
    let total = cases.len();
    Ok(OcelCasesResult { cases, total })
}

#[derive(Serialize)]
pub struct OcelObjectTypesResult {
    pub types: Vec<OcelObjectTypeSummary>,
}

/// List distinct OCEL object types and their instance counts.
#[verb("object-types")]
pub fn object_types() -> Result<OcelObjectTypesResult> {
    let svc = OcelService::new();
    let types = svc
        .object_types()
        .map_err(NounVerbError::execution_error)?;
    Ok(OcelObjectTypesResult { types })
}

#[derive(Serialize)]
pub struct OcelDiscoverResult {
    pub report: OcelDiscoveryReport,
    /// CANDIDATE when arcs exist; OPEN when the log has no transitions yet.
    pub status: String,
}

/// Discover an Object-Centric DFG from the mesh event log (OCEL 2.0).
#[verb("discover")]
pub fn discover() -> Result<OcelDiscoverResult> {
    let svc = OcelService::new();
    let report = svc.discover().map_err(NounVerbError::execution_error)?;
    let status = if report.arcs.is_empty() {
        "OPEN"
    } else {
        "CANDIDATE"
    }
    .to_string();
    Ok(OcelDiscoverResult { report, status })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_svc() -> (tempfile::NamedTempFile, OcelService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("ocel-inst"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = OcelService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // --- export ---

    #[test]
    fn export_returns_ok_for_valid_mesh() {
        let (_f, svc) = make_temp_svc();
        assert!(svc.export().is_ok());
    }

    #[test]
    fn export_contains_one_object_per_instance() {
        let (_f, svc) = make_temp_svc();
        let log = svc.export().unwrap();
        assert_eq!(log.objects.len(), 1);
        assert!(log.objects.contains_key("ocel-inst"));
    }

    #[test]
    fn export_object_type_is_lsp_instance() {
        let (_f, svc) = make_temp_svc();
        let log = svc.export().unwrap();
        let obj = log.objects.get("ocel-inst").unwrap();
        assert_eq!(obj.object_type, "LspInstance");
    }

    #[test]
    fn export_empty_event_log_produces_no_events() {
        let (_f, svc) = make_temp_svc();
        let log = svc.export().unwrap();
        assert!(
            log.events.is_empty(),
            "fresh instance has no hook events → no OCEL events"
        );
    }

    #[test]
    fn export_fails_on_missing_state_file() {
        let svc = OcelService {
            state_path: "/tmp/no-such-dir-lsp-max/ocel/state.json".to_string(),
        };
        assert!(svc.export().is_err());
    }

    // --- cases ---

    #[test]
    fn cases_no_events_returns_empty() {
        let (_f, svc) = make_temp_svc();
        let cases = svc.cases(None).unwrap();
        assert!(cases.is_empty(), "no events → no OCEL cases");
    }

    #[test]
    fn cases_fails_on_missing_state_file() {
        let svc = OcelService {
            state_path: "/tmp/no-such-dir-lsp-max/ocel-cases/state.json".to_string(),
        };
        assert!(svc.cases(None).is_err());
    }

    // --- object_types ---

    #[test]
    fn object_types_returns_lsp_instance_entry() {
        let (_f, svc) = make_temp_svc();
        let types = svc.object_types().unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].object_type, "LspInstance");
        assert_eq!(types[0].instance_count, 1);
    }

    #[test]
    fn object_types_fails_on_missing_state_file() {
        let svc = OcelService {
            state_path: "/tmp/no-such-dir-lsp-max/ocel-types/state.json".to_string(),
        };
        assert!(svc.object_types().is_err());
    }

    // --- discover ---

    #[test]
    fn discover_empty_log_returns_no_arcs() {
        let (_f, svc) = make_temp_svc();
        let report = svc.discover().unwrap();
        assert!(report.arcs.is_empty(), "no events → no OC-DFG arcs");
    }

    #[test]
    fn discover_reports_correct_object_count() {
        let (_f, svc) = make_temp_svc();
        let report = svc.discover().unwrap();
        assert_eq!(report.total_objects, 1);
    }

    #[test]
    fn discover_object_type_is_lsp_instance() {
        let (_f, svc) = make_temp_svc();
        let report = svc.discover().unwrap();
        assert!(report.object_types.contains(&"LspInstance".to_string()));
    }

    #[test]
    fn discover_fails_on_missing_state_file() {
        let svc = OcelService {
            state_path: "/tmp/no-such-dir-lsp-max/ocel-discover/state.json".to_string(),
        };
        assert!(svc.discover().is_err());
    }
}
