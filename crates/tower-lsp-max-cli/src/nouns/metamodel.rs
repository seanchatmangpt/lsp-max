use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct MetamodelResult {
    pub generated: bool,
}

#[verb("generate")]
pub fn cmd_generate() -> Result<MetamodelResult> {
    Ok(MetamodelResult { generated: true })
}

#[derive(Serialize)]
pub struct MetamodelInspectResult {
    pub inspected: bool,
}

#[verb("inspect")]
pub fn cmd_inspect() -> Result<MetamodelInspectResult> {
    Ok(MetamodelInspectResult { inspected: true })
}
