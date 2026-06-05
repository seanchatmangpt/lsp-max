use tower_lsp_max_protocol::AnalysisBundle;

pub struct AgentExporter;

impl AgentExporter {
    pub fn export_bundle(bundle: &AnalysisBundle) -> String {
        serde_json::to_string_pretty(bundle).unwrap()
    }
}
