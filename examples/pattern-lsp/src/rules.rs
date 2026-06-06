use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct RulesListResult {
    pub packs: Vec<String>,
}

#[derive(Serialize)]
pub struct RulesCheckResult {
    pub ok: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub pattern: String,
    pub path_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    pub message: String,
    pub rationale: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RulePack {
    pub rules: Vec<Rule>,
}

/// List all rules
#[verb("list")]
pub fn cmd_list() -> Result<RulesListResult> {
    Ok(RulesListResult { packs: vec![] })
}

/// Check rules
#[verb("check")]
pub fn cmd_check() -> Result<RulesCheckResult> {
    Ok(RulesCheckResult { ok: true })
}
