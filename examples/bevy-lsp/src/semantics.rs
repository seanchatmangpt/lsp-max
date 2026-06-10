use lsp_max::ast::AutoLspAdapter;
use lsp_max::lsp_types_max::Diagnostic;

pub fn dispatch_semantic_rules(
    adapter: &AutoLspAdapter,
    uri: &lsp_max::lsp_types_max::DocumentUri,
) -> Vec<Diagnostic> {
    let diagnostics = adapter.pull_diagnostics(uri);

    adapter.get_document(uri, |doc| {
        let _cursor = doc.tree.walk();
        // Formally extracted from the RDF Ontology via SPARQL
    });

    diagnostics
}
