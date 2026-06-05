use clap_noun_verb::{NounVerbError, Result};
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct MetamodelResult {
    pub generated: bool,
}

#[verb("generate")]
pub fn cmd_generate(
    input: Option<String>,
    output: Option<String>,
    include_proposed: Option<bool>,
) -> Result<MetamodelResult> {
    generate_metamodel(input, output, include_proposed)
}

#[derive(Serialize)]
pub struct MetamodelInspectResult {
    pub inspected: bool,
    pub version: String,
    pub requests_count: usize,
    pub notifications_count: usize,
    pub structures_count: usize,
    pub enumerations_count: usize,
    pub type_aliases_count: usize,
}

#[verb("inspect")]
pub fn cmd_inspect(input: String) -> Result<MetamodelInspectResult> {
    inspect_metamodel(input)
}

fn generate_metamodel(
    input: Option<String>,
    output: Option<String>,
    include_proposed: Option<bool>,
) -> Result<MetamodelResult> {
    let input_path = input.unwrap_or_else(|| {
        "/Users/sac/tower-lsp-max/crates/tower-lsp-max-specgen/fixtures/metaModel-3.18.json"
            .to_string()
    });
    let output_path = output.unwrap_or_else(|| {
        "/Users/sac/tower-lsp-max/tower-lsp-max-protocol/src/lsp_3_18.rs".to_string()
    });
    let include_prop = include_proposed.unwrap_or(false);

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run")
        .arg("--manifest-path")
        .arg("/Users/sac/tower-lsp-max/crates/tower-lsp-max-specgen/Cargo.toml")
        .arg("--")
        .arg("--input")
        .arg(&input_path)
        .arg("--output")
        .arg(&output_path);

    if include_prop {
        cmd.arg("--include-proposed");
    }

    let run_output = cmd
        .output()
        .map_err(|e| NounVerbError::execution_error(format!("Failed to run specgen: {}", e)))?;

    if !run_output.status.success() {
        return Err(NounVerbError::execution_error(format!(
            "Specgen generator failed: {}",
            String::from_utf8_lossy(&run_output.stderr)
        )));
    }

    Ok(MetamodelResult { generated: true })
}

fn inspect_metamodel(input: String) -> Result<MetamodelInspectResult> {
    let content = std::fs::read_to_string(&input).map_err(|e| {
        NounVerbError::execution_error(format!("Failed to read metamodel file: {}", e))
    })?;
    let val: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        NounVerbError::argument_error(format!("Failed to parse metamodel JSON: {}", e))
    })?;

    let version = val["metaData"]["version"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let requests_count = val["requests"].as_array().map(|a| a.len()).unwrap_or(0);

    let notifications_count = val["notifications"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    let structures_count = val["structures"].as_array().map(|a| a.len()).unwrap_or(0);

    let enumerations_count = val["enumerations"].as_array().map(|a| a.len()).unwrap_or(0);

    let type_aliases_count = val["typeAliases"].as_array().map(|a| a.len()).unwrap_or(0);

    Ok(MetamodelInspectResult {
        inspected: true,
        version,
        requests_count,
        notifications_count,
        structures_count,
        enumerations_count,
        type_aliases_count,
    })
}
