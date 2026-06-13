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

impl Default for ExtensionRouter {
    fn default() -> Self {
        Self::new()
    }
}
