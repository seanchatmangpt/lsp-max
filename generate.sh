#!/bin/bash
cd examples/clap-noun-verb-lsp/src

cat << 'INNER_EOF' > types.rs
use serde::Serialize;

#[derive(Serialize)]
pub struct CommandResult {
    pub success: bool,
}

pub struct CommandGraph;
pub struct NounNode;
pub struct VerbNode;
pub struct ArgNode;
pub struct TagNode;
pub struct RouteId;
pub struct CommandMoniker;
pub struct LayoutClassification;
pub struct DiagnosticFinding;
pub struct CodeActionPlan;
pub struct Receipt;

pub enum CliLayer {
    NounWrapper,
    Domain,
    Integration,
    Unknown,
}

pub enum RouteValidity {
    Valid,
    Duplicate,
    MissingVerb,
    MalformedVerb,
    DeprecatedNoun,
}

pub enum FakeCompletionSignal {
    InARealSystem,
    InProduction,
    Eventually,
    WouldBeImplemented,
    LeftAsExercise,
    MockOnly,
    Placeholder,
}
INNER_EOF

for file in analyzer.rs actions.rs diagnostics.rs edits.rs hierarchy.rs hints.rs inline_values.rs layer.rs monikers.rs receipts.rs symbols.rs tags.rs tokens.rs virtual_docs.rs workspace.rs; do
  touch "$file"
done

cat << 'INNER_EOF' > server.rs
use clap_noun_verb_macros::verb;
use clap_noun_verb::Result;
use crate::types::CommandResult;

#[verb("serve")]
pub fn cmd_serve(stdio: bool) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
INNER_EOF

cat << 'INNER_EOF' > command.rs
use clap_noun_verb_macros::verb;
use clap_noun_verb::Result;
use crate::types::CommandResult;

#[verb("inspect")]
pub fn cmd_inspect(noun: String, verb: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
INNER_EOF

cat << 'INNER_EOF' > doctor.rs
use clap_noun_verb_macros::verb;
use clap_noun_verb::Result;
use crate::types::CommandResult;

#[verb("check")]
pub fn cmd_check() -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
INNER_EOF

cat << 'INNER_EOF' > graph.rs
use clap_noun_verb_macros::verb;
use clap_noun_verb::Result;
use crate::types::CommandResult;

#[verb("export")]
pub fn cmd_export(format: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
INNER_EOF

cat << 'INNER_EOF' > layout.rs
use clap_noun_verb_macros::verb;
use clap_noun_verb::Result;
use crate::types::CommandResult;

#[verb("check")]
pub fn cmd_check() -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
INNER_EOF

cat << 'INNER_EOF' > rules.rs
use clap_noun_verb_macros::verb;
use clap_noun_verb::Result;
use crate::types::CommandResult;

#[verb("list")]
pub fn cmd_list() -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}

#[verb("check")]
pub fn cmd_check() -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
INNER_EOF

cat << 'INNER_EOF' > scan.rs
use clap_noun_verb_macros::verb;
use clap_noun_verb::Result;
use crate::types::CommandResult;

#[verb("workspace")]
pub fn cmd_workspace(format: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
INNER_EOF

