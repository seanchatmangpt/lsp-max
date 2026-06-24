// Registry types for lsp-max-compositor.
// The ExtensionRouter is populated at startup from lsp-max.toml via CompositorConfig::load().
// For generated initialization boilerplate, see ggen.toml (Phase 4 scaffold).
// Adding a new domain-specific server: add a [[server]] entry to lsp-max.toml — no Rust changes needed once the ggen template is implemented.

#[derive(Debug, Clone)]
pub enum ChildTier {
    Primary,
    Secondary,
    DiagnosticsOnly,
}

impl ChildTier {
    pub fn as_str(&self) -> &str {
        match self {
            ChildTier::Primary => "primary",
            ChildTier::Secondary => "secondary",
            ChildTier::DiagnosticsOnly => "diagnostics-only",
        }
    }

    /// Map a priority string from `lsp-max.toml` to a `ChildTier`.
    /// `"full"` and `"semantic"` are Primary; `"secondary"` is Secondary;
    /// everything else (including `"diagnostics-only"`) is DiagnosticsOnly.
    pub fn from_priority(priority: &str) -> Self {
        match priority {
            "full" | "semantic" => ChildTier::Primary,
            "secondary" => ChildTier::Secondary,
            _ => ChildTier::DiagnosticsOnly,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChildServer {
    pub id: String,
    pub tier: ChildTier,
    pub extensions: Vec<String>,
}

pub struct ExtensionRouter {
    inner: dashmap::DashMap<String, Vec<ChildServer>>,
    /// Workspace root this router was built for.  Two different workspace
    /// roots get isolated routers — this is the per-workspace stream
    /// isolation required by the L7 Speciation claim.
    pub workspace_root: Option<std::path::PathBuf>,
}

impl ExtensionRouter {
    pub fn new() -> Self {
        Self {
            inner: dashmap::DashMap::new(),
            workspace_root: None,
        }
    }

    pub fn with_workspace_root(root: std::path::PathBuf) -> Self {
        Self {
            inner: dashmap::DashMap::new(),
            workspace_root: Some(root),
        }
    }

    pub fn register(&self, ext: impl Into<String>, server: ChildServer) {
        self.inner.entry(ext.into()).or_default().push(server);
    }

    pub fn servers_for(&self, ext: &str) -> Vec<ChildServer> {
        self.inner.get(ext).map(|v| v.clone()).unwrap_or_default()
    }

    /// Return every distinct child server whose registered extension matches
    /// `filename`. A registered extension matches when `filename` ends with it
    /// after normalising a leading dot, so both bare keys (`rs`) and the dotted,
    /// possibly compound keys that `lsp-max.toml` declares (`.rs`, `.ocel.json`)
    /// resolve correctly. A server registered under several matching extensions
    /// (e.g. `.ocel.json` and `.json`) is returned once, deduplicated by id.
    ///
    /// This is the routing entry point: keying registration on `.rs` while
    /// looking up the bare `rs` was a silent miss, so config-driven routing
    /// resolved no servers at all before this match was introduced.
    pub fn servers_for_filename(&self, filename: &str) -> Vec<ChildServer> {
        let mut out: Vec<ChildServer> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for entry in self.inner.iter() {
            if extension_matches(filename, entry.key()) {
                for server in entry.value() {
                    if seen.insert(server.id.clone()) {
                        out.push(server.clone());
                    }
                }
            }
        }
        out
    }

    /// Build an `ExtensionRouter` from a [`crate::config::CompositorConfig`].
    ///
    /// For each server entry the priority string maps to tiers:
    /// - `"full"` → primary extensions get `ChildTier::Primary`, secondary get `ChildTier::Secondary`
    /// - `"diagnostics-only"` → all extensions get `ChildTier::DiagnosticsOnly`
    /// - anything else → treated as `"full"`
    pub fn from_config(config: &crate::config::CompositorConfig) -> Self {
        let router = Self::new();
        for entry in &config.server {
            let diagnostics_only = entry.priority == "diagnostics-only";
            let primary_tier = if diagnostics_only {
                ChildTier::DiagnosticsOnly
            } else {
                ChildTier::Primary
            };
            let secondary_tier = if diagnostics_only {
                ChildTier::DiagnosticsOnly
            } else {
                ChildTier::Secondary
            };
            for ext in &entry.primary_extensions {
                router.register(
                    ext.clone(),
                    ChildServer {
                        id: entry.id.clone(),
                        tier: primary_tier.clone(),
                        extensions: entry.primary_extensions.clone(),
                    },
                );
            }
            for ext in &entry.secondary_extensions {
                router.register(
                    ext.clone(),
                    ChildServer {
                        id: entry.id.clone(),
                        tier: secondary_tier.clone(),
                        extensions: entry.secondary_extensions.clone(),
                    },
                );
            }
        }
        router
    }
}

/// True when `filename` carries the dotted extension `ext`. A missing leading
/// dot on `ext` is normalised, so `"rs"` and `".rs"` both match `"main.rs"`,
/// and a real dot boundary is required so `"rs"` does not match `"foors"`.
/// Compound extensions such as `".ocel.json"` match `"log.ocel.json"`.
fn extension_matches(filename: &str, ext: &str) -> bool {
    if ext.is_empty() {
        return false;
    }
    if ext.starts_with('.') {
        filename.ends_with(ext)
    } else {
        filename.len() > ext.len()
            && filename.ends_with(ext)
            && filename.as_bytes()[filename.len() - ext.len() - 1] == b'.'
    }
}

impl Default for ExtensionRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(id: &str, exts: &[&str]) -> ChildServer {
        ChildServer {
            id: id.into(),
            tier: ChildTier::DiagnosticsOnly,
            extensions: exts.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn extension_matches_handles_dot_normalisation_and_boundaries() {
        assert!(extension_matches("main.rs", ".rs"));
        assert!(extension_matches("main.rs", "rs")); // bare key still matches
        assert!(extension_matches("log.ocel.json", ".ocel.json")); // compound
        assert!(extension_matches("log.ocel.json", ".json")); // generic suffix too
        assert!(!extension_matches("foors", "rs")); // no dot boundary
        assert!(!extension_matches("foo.tsx", ".ts")); // tsx is not ts
        assert!(!extension_matches("anything", ""));
    }

    #[test]
    fn config_format_dotted_key_routes_multiple_servers_to_one_extension() {
        // The multi-LSP-per-extension case: two diagnostic servers share `.rs`.
        let router = ExtensionRouter::new();
        router.register(".rs", diag("wasm4pm-lsp", &[".rs"]));
        router.register(".rs", diag("anti-llm-cheat-lsp", &[".rs"]));

        let mut ids: Vec<String> = router
            .servers_for_filename("main.rs")
            .into_iter()
            .map(|s| s.id)
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["anti-llm-cheat-lsp", "wasm4pm-lsp"]);
    }

    #[test]
    fn compound_and_generic_match_dedup_same_server() {
        // wasm4pm-lsp is registered under both `.ocel.json` and `.json`; a
        // `.ocel.json` file matches both keys but the server appears once.
        let router = ExtensionRouter::new();
        router.register(".ocel.json", diag("wasm4pm-lsp", &[".ocel.json", ".json"]));
        router.register(".json", diag("wasm4pm-lsp", &[".ocel.json", ".json"]));

        let servers = router.servers_for_filename("trace.ocel.json");
        assert_eq!(servers.len(), 1, "server must be deduplicated across keys");
        assert_eq!(servers[0].id, "wasm4pm-lsp");
    }
}
