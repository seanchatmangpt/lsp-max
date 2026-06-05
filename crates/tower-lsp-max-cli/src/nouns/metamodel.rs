use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==========================================
// Tier 1: Domain
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct MetamodelEntity {
    pub id: String,
    pub version: String,
    pub structure_hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    pub path: String,
    pub change_type: String,
}

// ==========================================
// Tier 2: Service
// ==========================================

pub struct MetamodelService;

impl MetamodelService {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, version: &str) -> MetamodelEntity {
        MetamodelEntity {
            id: format!("meta-{}", version),
            version: version.to_string(),
            structure_hash: "mock_hash_xyz123".to_string(),
        }
    }

    pub fn inspect(&self, id: &str) -> MetamodelEntity {
        MetamodelEntity {
            id: id.to_string(),
            version: "3.18".to_string(),
            structure_hash: "mock_hash_xyz123".to_string(),
        }
    }

    pub fn validate(&self, _id: &str) -> Vec<ValidationIssue> {
        vec![ValidationIssue {
            level: "INFO".to_string(),
            message: "Metamodel is structurally sound.".to_string(),
        }]
    }

    pub fn diff(&self, _source_id: &str, _target_id: &str) -> Vec<DiffEntry> {
        vec![DiffEntry {
            path: "$.structures.ClientCapabilities".to_string(),
            change_type: "MODIFIED".to_string(),
        }]
    }
}

impl Default for MetamodelService {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// Tier 3: CLI Verbs & Results
// ==========================================

#[derive(Serialize)]
pub struct GenerateResult {
    pub entity: MetamodelEntity,
}

#[verb("generate")]
pub fn generate(version: String) -> Result<GenerateResult> {
    let service = MetamodelService::new();
    let entity = service.generate(&version);
    Ok(GenerateResult { entity })
}

#[derive(Serialize)]
pub struct InspectResult {
    pub entity: MetamodelEntity,
}

#[verb("inspect")]
pub fn inspect(id: String) -> Result<InspectResult> {
    let service = MetamodelService::new();
    let entity = service.inspect(&id);
    Ok(InspectResult { entity })
}

#[derive(Serialize)]
pub struct ValidateResult {
    pub issues: Vec<ValidationIssue>,
    pub is_valid: bool,
}

#[verb("validate")]
pub fn validate(id: String) -> Result<ValidateResult> {
    let service = MetamodelService::new();
    let issues = service.validate(&id);
    let is_valid = issues.iter().all(|i| i.level != "ERROR");
    Ok(ValidateResult { issues, is_valid })
}

#[derive(Serialize)]
pub struct DiffResult {
    pub diffs: Vec<DiffEntry>,
}

#[verb("diff")]
pub fn diff(source_id: String, target_id: String) -> Result<DiffResult> {
    let service = MetamodelService::new();
    let diffs = service.diff(&source_id, &target_id);
    Ok(DiffResult { diffs })
}
