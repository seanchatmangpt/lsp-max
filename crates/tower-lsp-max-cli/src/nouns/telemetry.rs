use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct TelemetryResult {
    pub exported: bool,
}

#[derive(Serialize)]
pub struct TraceResult {
    pub traced: bool,
}

#[verb("export")]
pub fn cmd_export() -> Result<TelemetryResult> {
    Ok(TelemetryResult { exported: true })
}

#[verb("trace")]
pub fn cmd_trace() -> Result<TraceResult> {
    Ok(TraceResult { traced: true })
}
