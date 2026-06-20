use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct OntologyStatusResult {
    pub ontology_dir: String,
    pub files_found: Vec<String>,
    pub method_count: usize,
    pub admitted_count: usize,
    pub refused_count: usize,
    pub candidate_count: usize,
    pub unknown_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OntologyValidateResult {
    pub valid: bool,
    pub issues: Vec<String>,
    pub checked_files: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OntologyExportResult {
    pub format: String,
    pub output_path: String,
    pub triple_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OntologyAssertResult {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub file: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OntologyQueryResult {
    pub query: String,
    pub sparql_file: Option<String>,
    pub result_hint: String,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct OntologyService;

impl OntologyService {
    pub fn new() -> Self {
        Self
    }

    fn ontology_dir(dir: &str) -> PathBuf {
        PathBuf::from(dir).join("ontology")
    }

    fn count_pattern(content: &str, pattern: &str) -> usize {
        content.matches(pattern).count()
    }

    pub fn status(&self, dir: &str) -> OntologyStatusResult {
        let onto_dir = Self::ontology_dir(dir);
        let mut files_found = vec![];
        let mut method_count = 0;
        let mut admitted_count = 0;
        let mut refused_count = 0;
        let mut candidate_count = 0;
        let mut unknown_count = 0;

        if let Ok(entries) = fs::read_dir(&onto_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("ttl") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        method_count += Self::count_pattern(&content, "lsp:methodName");
                        admitted_count += Self::count_pattern(&content, "law:ADMITTED");
                        refused_count += Self::count_pattern(&content, "law:REFUSED");
                        candidate_count += Self::count_pattern(&content, "law:CANDIDATE");
                        unknown_count += Self::count_pattern(&content, "law:UNKNOWN");
                    }
                    files_found.push(
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }

        OntologyStatusResult {
            ontology_dir: onto_dir.display().to_string(),
            files_found,
            method_count,
            admitted_count,
            refused_count,
            candidate_count,
            unknown_count,
            status: "CANDIDATE".into(),
        }
    }

    pub fn validate(&self, dir: &str) -> OntologyValidateResult {
        let onto_dir = Self::ontology_dir(dir);
        let mut issues = vec![];
        let mut checked = vec![];

        let required = ["lsp318.ttl", "law-axes.ttl", "max-protocol.ttl"];
        for fname in required {
            let p = onto_dir.join(fname);
            if p.exists() {
                checked.push(fname.to_string());
            } else {
                issues.push(format!("OPEN: missing required file ontology/{fname}"));
            }
        }

        OntologyValidateResult {
            valid: issues.is_empty(),
            issues,
            checked_files: checked,
            status: "CANDIDATE".into(),
        }
    }

    pub fn export(&self, dir: &str, format: &str, output: &str) -> Result<OntologyExportResult> {
        let onto_dir = Self::ontology_dir(dir);
        let mut all_content = String::new();
        let mut triple_count = 0;

        if let Ok(entries) = fs::read_dir(&onto_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("ttl") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        triple_count += content
                            .lines()
                            .filter(|l| l.contains(';') || l.contains('.'))
                            .count();
                        all_content.push_str(&content);
                        all_content.push('\n');
                    }
                }
            }
        }

        fs::write(output, &all_content).map_err(|e| {
            clap_noun_verb::error::NounVerbError::execution_error(e.to_string())
        })?;

        Ok(OntologyExportResult {
            format: format.to_string(),
            output_path: output.to_string(),
            triple_count,
            status: "CANDIDATE".into(),
        })
    }

    pub fn assert_triple(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        file: &str,
    ) -> Result<OntologyAssertResult> {
        let triple = format!("{subject} {predicate} {object} .\n");
        let mut content = fs::read_to_string(file).unwrap_or_default();
        if !content.contains(&triple) {
            content.push_str(&triple);
            fs::write(file, &content).map_err(|e| {
                clap_noun_verb::error::NounVerbError::execution_error(e.to_string())
            })?;
        }
        Ok(OntologyAssertResult {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            file: file.to_string(),
            status: "CANDIDATE".into(),
        })
    }

    pub fn query(
        &self,
        sparql_file: Option<&str>,
        inline: Option<&str>,
    ) -> OntologyQueryResult {
        OntologyQueryResult {
            query: inline.unwrap_or("(see sparql_file)").to_string(),
            sparql_file: sparql_file.map(str::to_string),
            result_hint: "Run `ggen sync` to execute SPARQL queries against the ontology"
                .into(),
            status: "OPEN".into(),
        }
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[verb("status")]
pub fn status(dir: Option<String>) -> Result<OntologyStatusResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(OntologyService::new().status(&dir))
}

#[verb("validate")]
pub fn validate(dir: Option<String>) -> Result<OntologyValidateResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(OntologyService::new().validate(&dir))
}

#[verb("export")]
pub fn export(
    dir: Option<String>,
    format: Option<String>,
    output: Option<String>,
) -> Result<OntologyExportResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    let format = format.unwrap_or_else(|| "turtle".to_string());
    let output = output.unwrap_or_else(|| "ontology-merged.ttl".to_string());
    OntologyService::new().export(&dir, &format, &output)
}

#[verb("assert")]
pub fn assert_triple(
    subject: String,
    predicate: String,
    object: String,
    file: Option<String>,
) -> Result<OntologyAssertResult> {
    let file = file.unwrap_or_else(|| "ontology/domain.ttl".to_string());
    OntologyService::new().assert_triple(&subject, &predicate, &object, &file)
}

#[verb("query")]
pub fn query(
    sparql_file: Option<String>,
    inline: Option<String>,
) -> Result<OntologyQueryResult> {
    Ok(OntologyService::new().query(sparql_file.as_deref(), inline.as_deref()))
}
