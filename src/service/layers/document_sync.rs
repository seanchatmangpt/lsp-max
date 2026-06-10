use dashmap::DashMap;
use futures::future::BoxFuture;
use futures::lock::Mutex;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceExt};
use url::Url;

use super::super::ExitedError;
use crate::jsonrpc::{Request, Response};

/// Middleware which serializes requests/notifications based on their document URI.
#[derive(Clone)]
pub struct DocumentSync {
    locks: Arc<DashMap<Url, Arc<Mutex<()>>>>,
}

impl DocumentSync {
    /// Creates a new `DocumentSync` layer.
    pub fn new() -> Self {
        DocumentSync {
            locks: Arc::new(DashMap::new()),
        }
    }
}

impl Default for DocumentSync {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for DocumentSync {
    type Service = DocumentSyncService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DocumentSyncService {
            inner,
            locks: self.locks.clone(),
        }
    }
}

/// Service created from [`DocumentSync`] layer.
#[derive(Clone)]
pub struct DocumentSyncService<S> {
    inner: S,
    locks: Arc<DashMap<Url, Arc<Mutex<()>>>>,
}

impl<S> Service<Request> for DocumentSyncService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let uri = extract_uri(req.params());
        let method = req.method().to_string();
        let params = req.params().cloned();
        let locks = self.locks.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let is_read_query = match method.as_str() {
                "textDocument/hover"
                | "textDocument/completion"
                | "textDocument/definition"
                | "textDocument/references"
                | "textDocument/documentHighlight"
                | "textDocument/documentSymbol"
                | "textDocument/codeAction"
                | "textDocument/codeLens"
                | "textDocument/documentLink"
                | "textDocument/formatting"
                | "textDocument/rangeFormatting"
                | "textDocument/onTypeFormatting"
                | "textDocument/rename"
                | "textDocument/prepareRename"
                | "textDocument/foldingRange"
                | "textDocument/selectionRange"
                | "textDocument/prepareCallHierarchy"
                | "textDocument/callHierarchyIncomingCalls"
                | "textDocument/callHierarchyOutgoingCalls"
                | "textDocument/inlayHint"
                | "textDocument/inlineValue"
                | "textDocument/moniker" => true,
                m if m.starts_with("textDocument/semanticTokens") => true,
                _ => false,
            };
            let should_lock = !is_read_query;

            if let Some(uri) = uri {
                if should_lock {
                    let lock = locks
                        .entry(uri.clone())
                        .or_insert_with(|| Arc::new(Mutex::new(())))
                        .value()
                        .clone();
                    let _guard = lock.lock().await;

                    // Update version if this is a didOpen or didChange notification
                    match method.as_str() {
                        "textDocument/didOpen" => {
                            if let Some(version) = params
                                .as_ref()
                                .and_then(|p| p.get("textDocument"))
                                .and_then(|t| t.get("version"))
                                .and_then(|v| v.as_i64())
                            {
                                if let Ok(mut reg) = crate::get_registry().lock() {
                                    reg.document_versions.insert(uri, version as i32);
                                }
                            }
                        }
                        "textDocument/didChange" => {
                            if let Some(version) = params
                                .as_ref()
                                .and_then(|p| p.get("textDocument"))
                                .and_then(|t| t.get("version"))
                                .and_then(|v| v.as_i64())
                            {
                                if let Ok(mut reg) = crate::get_registry().lock() {
                                    reg.document_versions.insert(uri, version as i32);
                                }
                            }
                        }
                        _ => {}
                    }

                    inner.ready().await?.call(req).await
                } else {
                    inner.ready().await?.call(req).await
                }
            } else {
                inner.ready().await?.call(req).await
            }
        })
    }
}

fn extract_uri(params: Option<&serde_json::Value>) -> Option<Url> {
    let params = params?;
    if let Some(uri) = params
        .get("textDocument")
        .and_then(|t| t.get("uri"))
        .and_then(|u| u.as_str())
    {
        if let Ok(url) = Url::parse(uri) {
            return Some(url);
        }
    }
    if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
        if let Ok(url) = Url::parse(uri) {
            return Some(url);
        }
    }
    None
}
