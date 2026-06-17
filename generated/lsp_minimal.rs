// Generated from fixtures/minimal-metaModel.json — do not edit by hand.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentDiagnosticParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
}
