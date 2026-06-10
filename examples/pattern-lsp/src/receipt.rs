use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct ReceiptShowResult {
    pub receipt_id: String,
}

/// Show receipt
/// # Arguments
/// * `latest` - Show the latest receipt
#[verb("show")]
pub fn cmd_show(latest: bool) -> Result<ReceiptShowResult> {
    let _ = latest;
    Ok(ReceiptShowResult {
        receipt_id: "none".into(),
    })
}
