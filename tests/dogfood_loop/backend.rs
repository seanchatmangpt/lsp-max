/// DogfoodBackend and related types for the dogfood loop tests.
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_lsp_max::jsonrpc::Result;
use tower_lsp_max::lsp_types as lsp;
use tower_lsp_max::max_protocol::lsp_3_18 as lsp318;
use tower_lsp_max::{Client, LanguageServer};

#[derive(Clone, Default)]
pub struct TestEvents {
    pub did_open_notebook: Arc<Mutex<Option<lsp::DidOpenNotebookDocumentParams>>>,
    pub set_trace: Arc<Mutex<Option<lsp::SetTraceParams>>>,
    pub progress: Arc<Mutex<Option<lsp::ProgressParams>>>,
    pub work_done_cancel: Arc<Mutex<Option<lsp::WorkDoneProgressCancelParams>>>,
}

pub struct DogfoodBackend {
    pub client: Client,
    pub events: TestEvents,
}

#[tower_lsp_max::async_trait]
impl LanguageServer for DogfoodBackend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        println!("Server: initialize called");
        Ok(lsp::InitializeResult::default())
    }

    async fn initialized(&self, _: lsp::InitializedParams) {
        println!("Server: initialized called");
        let client = self.client.clone();
        tokio::spawn(async move {
            println!("Server task: starting sleep");
            tokio::time::sleep(Duration::from_millis(50)).await;

            println!("Server task: sending work_done_progress_create");
            let create_res = client
                .work_done_progress_create(lsp::WorkDoneProgressCreateParams {
                    token: lsp::NumberOrString::String("dogfood-progress-token".to_string()),
                })
                .await;
            assert!(
                create_res.is_ok(),
                "Server failed to send window/workDoneProgress/create request: {:?}",
                create_res
            );

            println!("Server task: sending folding_range_refresh");
            let refresh_res = client.folding_range_refresh().await;
            assert!(
                refresh_res.is_ok(),
                "Server failed to send workspace/foldingRange/refresh request: {:?}",
                refresh_res
            );

            println!("Server task: sending text_document_content_refresh");
            let refresh_content_res = client
                .text_document_content_refresh(lsp318::TextDocumentContentRefreshParams {
                    uri: "file:///dogfood.rs".to_string(),
                })
                .await;
            assert!(
                refresh_content_res.is_ok(),
                "Server failed to send workspace/textDocumentContent/refresh request: {:?}",
                refresh_content_res
            );

            println!("Server task: sending log_trace");
            client
                .log_trace(lsp::LogTraceParams {
                    message: "dogfood log trace message".to_string(),
                    verbose: Some("dogfood log trace verbose content".to_string()),
                })
                .await;
            println!("Server task: log_trace completed");
        });
    }

    async fn shutdown(&self) -> Result<()> {
        println!("Server: shutdown called");
        Ok(())
    }

    async fn inline_completion(
        &self,
        params: lsp::InlineCompletionParams,
    ) -> Result<Option<lsp::InlineCompletionResponse>> {
        println!("Server: inline_completion called");
        assert_eq!(
            params.text_document_position.text_document.uri.as_str(),
            "file:///dogfood.rs"
        );
        Ok(Some(lsp::InlineCompletionResponse::List(
            lsp::InlineCompletionList {
                items: vec![lsp::InlineCompletionItem {
                    insert_text: lsp::StringOrStringValue::String(
                        "dogfood_inline_completion_text".to_string(),
                    ),
                    filter_text: None,
                    range: None,
                    command: None,
                    insert_text_format: None,
                }],
            },
        )))
    }

    async fn text_document_content(
        &self,
        params: lsp318::TextDocumentContentParams,
    ) -> Result<lsp318::TextDocumentContentResult> {
        println!("Server: text_document_content called");
        assert_eq!(params.text_document.uri.as_str(), "file:///dogfood.rs");
        Ok(lsp318::TextDocumentContentResult {
            text: "dogfood document content text".to_string(),
        })
    }

    async fn ranges_formatting(
        &self,
        params: lsp318::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<lsp318::TextEdit>>> {
        println!("Server: ranges_formatting called");
        assert_eq!(params.text_document.uri.as_str(), "file:///dogfood.rs");
        Ok(Some(vec![lsp318::TextEdit {
            range: lsp318::Range {
                start: lsp318::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp318::Position {
                    line: 0,
                    character: 10,
                },
            },
            new_text: "formatted_dogfood_ranges".to_string(),
        }]))
    }

    async fn did_open_notebook_document(&self, params: lsp::DidOpenNotebookDocumentParams) {
        println!("Server: did_open_notebook_document called");
        *self.events.did_open_notebook.lock().unwrap() = Some(params);
    }

    async fn set_trace(&self, params: lsp::SetTraceParams) {
        println!("Server: set_trace called");
        *self.events.set_trace.lock().unwrap() = Some(params);
    }

    async fn progress(&self, params: lsp::ProgressParams) {
        println!("Server: progress called");
        *self.events.progress.lock().unwrap() = Some(params);
    }

    async fn work_done_progress_cancel(&self, params: lsp::WorkDoneProgressCancelParams) {
        println!("Server: work_done_progress_cancel called");
        *self.events.work_done_cancel.lock().unwrap() = Some(params);
    }
}
