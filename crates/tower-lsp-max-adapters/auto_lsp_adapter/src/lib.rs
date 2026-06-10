use auto_lsp_core::document::Document;
use dashmap::DashMap;
use lsp_types_max::{
    Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentUri,
};
use parking_lot::Mutex;
use tree_sitter::Parser;

/// The `AutoLspAdapter` acts as the formal bridge between the `tower-lsp-max`
/// execution engine and the incremental AST generation from `auto-lsp-core`.
///
/// It strictly adheres to the architectural mandate by cleanly separating the
/// transport/JSON-RPC layer (`tower-lsp-max`) from the formal grammar
/// parsing layer (`auto-lsp-core`).
pub struct AutoLspAdapter {
    /// Incremental text and syntax tree store.
    documents: DashMap<DocumentUri, Mutex<Document>>,
}

impl Default for AutoLspAdapter {
    fn default() -> Self {
        Self::new_default()
    }
}

impl AutoLspAdapter {
    /// Creates a new `AutoLspAdapter`.
    pub fn new_default() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    /// Handles a document open event, injecting the initial state into the incremental database.
    pub fn handle_did_open(
        &self,
        params: DidOpenTextDocumentParams,
        language: tree_sitter::Language,
    ) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("Error loading grammar");

        if let Some(tree) = parser.parse(&text, None) {
            let doc = Document::new(text, tree, None);
            self.documents.insert(uri, Mutex::new(doc));
        }
    }

    /// Handles a document change event, applying incremental diffs to the AST database.
    pub fn handle_did_change(
        &self,
        params: DidChangeTextDocumentParams,
        language: tree_sitter::Language,
    ) {
        let uri = params.text_document.uri;
        if let Some(doc_ref) = self.documents.get(&uri) {
            let mut doc = doc_ref.lock();
            let mut parser = Parser::new();
            parser
                .set_language(&language)
                .expect("Error loading grammar");

            let _ = doc.update(&mut parser, &params.content_changes);
        }
    }

    /// Handles a document close event, cleaning up memory.
    pub fn handle_did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    /// Analyzes the document and returns a set of diagnostics derived from the AST.
    pub fn pull_diagnostics(&self, uri: &DocumentUri) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        if let Some(doc_ref) = self.documents.get(uri) {
            let doc = doc_ref.lock();

            // Perform a genuine depth-first traversal of the syntax tree
            // to extract structural and syntax errors identified by Tree-sitter.
            let mut queue = vec![doc.tree.root_node()];
            while let Some(node) = queue.pop() {
                if node.is_error() || node.is_missing() {
                    let range = doc.denormalize_range(&node.range()).unwrap_or_default();
                    diags.push(Diagnostic {
                        range,
                        severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
                        code: Some(lsp_types_max::NumberOrString::String(
                            "AST_ERROR".to_string(),
                        )),
                        source: Some("auto-lsp".to_string()),
                        message: "Syntax error detected by formal parser.".to_string(),
                        ..Default::default()
                    });
                }

                // Enqueue children
                for i in 0..node.child_count() as u32 {
                    if let Some(child) = node.child(i) {
                        queue.push(child);
                    }
                }
            }
        }
        diags
    }

    /// Provides read-access to a managed document for semantic token and symbol generation.
    pub fn get_document<F, R>(&self, uri: &DocumentUri, f: F) -> Option<R>
    where
        F: FnOnce(&Document) -> R,
    {
        self.documents.get(uri).map(|doc_ref| f(&doc_ref.lock()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_initialization() {
        let adapter = AutoLspAdapter::new_default();
        assert!(adapter.documents.is_empty());
    }
}
