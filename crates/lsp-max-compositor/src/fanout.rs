// Fan-out dispatcher — routes LSP events to all registered child servers.
// Phase 2: wire AutonomicMesh hook dispatch to ExtensionRouter child list.

use crate::registry::{ChildServer, ChildTier, ExtensionRouter};

/// How a given LSP method should be dispatched across child servers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchStrategy {
    /// First Primary server that returns a non-null result wins; others are skipped.
    FirstSuccess,
    /// All servers contribute; results are merged (diagnostics union, etc.).
    FanAll,
    /// Notification: fan to all servers, no response merge needed.
    Notify,
    /// Send only to Primary-tier servers; Secondary/DiagnosticsOnly are skipped.
    PrimaryOnly,
}

/// Classify an LSP method name into a dispatch strategy.
pub fn dispatch_strategy(method: &str) -> DispatchStrategy {
    match method {
        "textDocument/hover"
        | "textDocument/completion"
        | "textDocument/definition"
        | "textDocument/declaration"
        | "textDocument/implementation"
        | "textDocument/references"
        | "textDocument/documentSymbol" => DispatchStrategy::FirstSuccess,

        "textDocument/publishDiagnostics" | "textDocument/diagnostic" => DispatchStrategy::FanAll,

        "textDocument/didOpen"
        | "textDocument/didChange"
        | "textDocument/didClose"
        | "textDocument/didSave" => DispatchStrategy::Notify,

        _ => DispatchStrategy::PrimaryOnly,
    }
}

/// Extract the filename (last path segment) from a URI string
/// (e.g. `"file:///foo/bar.ocel.json"` → `"bar.ocel.json"`).
/// Strips any query/fragment first. Returns the whole input if it has no `/`.
fn filename_from_uri(uri: &str) -> &str {
    let path = uri.split('?').next().unwrap_or(uri);
    let path = path.split('#').next().unwrap_or(path);
    path.rsplit('/').next().unwrap_or(path)
}

/// Return the ordered list of child servers that should receive a message for
/// the given file URI.  Order: Primary first, then Secondary, then DiagnosticsOnly.
///
/// Matching is by dotted-extension suffix over the filename, so the dotted and
/// compound keys declared in `lsp-max.toml` (`.rs`, `.ocel.json`) resolve, and a
/// shared extension fans out to every server registered for it.
pub fn servers_for_uri(router: &ExtensionRouter, uri: &str) -> Vec<ChildServer> {
    let filename = filename_from_uri(uri);
    let mut servers = router.servers_for_filename(filename);

    servers.sort_by_key(|s| match s.tier {
        ChildTier::Primary => 0u8,
        ChildTier::Secondary => 1,
        ChildTier::DiagnosticsOnly => 2,
        // Lsif is a fallback tier; sort after DiagnosticsOnly.
        ChildTier::Lsif => 3,
    });

    servers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ChildServer, ChildTier, ExtensionRouter};

    #[test]
    fn filename_extracted_from_file_uri() {
        assert_eq!(filename_from_uri("file:///workspace/main.rs"), "main.rs");
        assert_eq!(
            filename_from_uri("file:///workspace/log.ocel.json"),
            "log.ocel.json"
        );
        assert_eq!(filename_from_uri("file:///workspace/noext"), "noext");
    }

    #[test]
    fn first_success_candidates_are_primary_only() {
        // hover/completion/definition dispatch filters servers_for_uri to Primary;
        // a DiagnosticsOnly co-tenant on the same extension is not a candidate.
        let router = ExtensionRouter::new();
        router.register(
            ".rs",
            ChildServer {
                id: "ggen-lsp".into(),
                tier: ChildTier::Primary,
                extensions: vec![".rs".into()],
            },
        );
        router.register(
            ".rs",
            ChildServer {
                id: "diagnostics-only-lsp".into(),
                tier: ChildTier::DiagnosticsOnly,
                extensions: vec![".rs".into()],
            },
        );

        let primary: Vec<String> = servers_for_uri(&router, "file:///w/main.rs")
            .into_iter()
            .filter(|s| matches!(s.tier, ChildTier::Primary))
            .map(|s| s.id)
            .collect();
        assert_eq!(primary, vec!["ggen-lsp"]);
    }

    #[test]
    fn strategy_classification() {
        assert_eq!(
            dispatch_strategy("textDocument/hover"),
            DispatchStrategy::FirstSuccess
        );
        assert_eq!(
            dispatch_strategy("textDocument/diagnostic"),
            DispatchStrategy::FanAll
        );
        assert_eq!(
            dispatch_strategy("textDocument/didOpen"),
            DispatchStrategy::Notify
        );
        assert_eq!(
            dispatch_strategy("textDocument/formatting"),
            DispatchStrategy::PrimaryOnly
        );
    }

    #[test]
    fn servers_ordered_by_tier() {
        let router = ExtensionRouter::new();
        router.register(
            "rs",
            ChildServer {
                id: "diag".into(),
                tier: ChildTier::DiagnosticsOnly,
                extensions: vec!["rs".into()],
            },
        );
        router.register(
            "rs",
            ChildServer {
                id: "primary".into(),
                tier: ChildTier::Primary,
                extensions: vec!["rs".into()],
            },
        );
        router.register(
            "rs",
            ChildServer {
                id: "secondary".into(),
                tier: ChildTier::Secondary,
                extensions: vec!["rs".into()],
            },
        );

        let result = servers_for_uri(&router, "file:///workspace/main.rs");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, "primary");
        assert_eq!(result[1].id, "secondary");
        assert_eq!(result[2].id, "diag");
    }
}
