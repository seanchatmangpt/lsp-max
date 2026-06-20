use crate::lsp_3_18::{TextDocumentContentRefreshParams, TextDocumentContentRefreshRequest};

impl lsp_types_max::request::Request for TextDocumentContentRefreshRequest {
    type Params = TextDocumentContentRefreshParams;
    type Result = ();
    const METHOD: &'static str = "workspace/textDocumentContent/refresh";
}
