use super::types::GraphAdmissionError;

pub fn resolve_db_path() -> std::path::PathBuf {
    if let Ok(path_str) = std::env::var("LSP_MAX_DB_PATH") {
        if !path_str.trim().is_empty() {
            return std::path::PathBuf::from(path_str);
        }
    }

    let config_path = if let Ok(path_str) = std::env::var("LSP_MAX_CONFIG") {
        Some(std::path::PathBuf::from(path_str))
    } else if let Ok(home) = std::env::var("HOME") {
        Some(std::path::PathBuf::from(home).join(".lsp-max-config.json"))
    } else {
        Some(std::path::PathBuf::from(".lsp-max-config.json"))
    };

    if let Some(path) = config_path {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(db_path_val) = value.get("database_path").and_then(|v| v.as_str()) {
                        if !db_path_val.trim().is_empty() {
                            return std::path::PathBuf::from(db_path_val);
                        }
                    }
                }
            }
        }
    }

    let is_test =
        std::env::var("CARGO_MANIFEST_DIR").is_ok() || std::env::var("LSP_MAX_TEST").is_ok();

    if is_test {
        let temp_dir = std::env::temp_dir();
        temp_dir.join(format!("lsp-max-db-{}", uuid::Uuid::new_v4()))
    } else if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".local/share/lsp-max/db")
    } else {
        std::env::temp_dir().join("lsp-max-db")
    }
}

pub fn validate_shacl_shapes(quads: &[oxigraph::model::Quad]) -> Result<(), GraphAdmissionError> {
    for quad in quads {
        let predicate_str = quad.predicate.as_str();
        let object_str = match &quad.object {
            oxigraph::model::Term::Literal(l) => l.value(),
            oxigraph::model::Term::NamedNode(n) => n.as_str(),
            oxigraph::model::Term::BlankNode(b) => b.as_str(),
            oxigraph::model::Term::Triple(t) => {
                panic!("RDF-star triples are not supported in validation: {:?}", t)
            }
        };

        // Severity validation
        if predicate_str == "urn:lsp-max:core:severity"
            && object_str != "error"
            && object_str != "warning"
            && object_str != "info"
            && object_str != "hint"
        {
            return Err(GraphAdmissionError::NamespaceViolation(format!(
                "SHACL PropertyShape violation: invalid severity value '{}'",
                object_str
            )));
        }

        // Datatype validation for line/char numbers
        if (predicate_str == "urn:lsp-max:core:startLine"
            || predicate_str == "urn:lsp-max:core:startCharacter"
            || predicate_str == "urn:lsp-max:core:endLine"
            || predicate_str == "urn:lsp-max:core:endCharacter")
            && object_str.parse::<u32>().is_err()
        {
            return Err(GraphAdmissionError::ParsingFailed(format!(
                "SHACL PropertyShape violation: expected integer for {}, got '{}'",
                predicate_str, object_str
            )));
        }

        // positionEncoding validation
        if predicate_str == "urn:lsp-max:core:positionEncoding"
            && object_str != "utf-8"
            && object_str != "utf-16"
            && object_str != "utf-32"
        {
            return Err(GraphAdmissionError::NamespaceViolation(format!(
                "SHACL PropertyShape violation: invalid positionEncoding '{}'",
                object_str
            )));
        }

        // version validation
        if predicate_str == "urn:lsp-max:core:version" && object_str.is_empty() {
            return Err(GraphAdmissionError::NamespaceViolation(
                "SHACL PropertyShape violation: version is empty".to_string(),
            ));
        }

        // projectRoot validation
        if predicate_str == "urn:lsp-max:core:projectRoot" && object_str.is_empty() {
            return Err(GraphAdmissionError::NamespaceViolation(
                "SHACL PropertyShape violation: projectRoot is empty".to_string(),
            ));
        }

        // uri validation
        if predicate_str == "urn:lsp-max:core:uri" && object_str.is_empty() {
            return Err(GraphAdmissionError::NamespaceViolation(
                "SHACL PropertyShape violation: uri is empty".to_string(),
            ));
        }

        // languageId validation
        if predicate_str == "urn:lsp-max:core:languageId" && object_str.is_empty() {
            return Err(GraphAdmissionError::NamespaceViolation(
                "SHACL PropertyShape violation: languageId is empty".to_string(),
            ));
        }
    }
    Ok(())
}

pub struct StoreFactory;

impl StoreFactory {
    pub fn open() -> Result<oxigraph::store::Store, String> {
        let path = resolve_db_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::create_dir_all(&path);
        oxigraph::store::Store::new().map_err(|e| e.to_string())
    }
}
