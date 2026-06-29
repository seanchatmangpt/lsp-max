#[derive(Debug, serde::Deserialize)]
pub struct CompositorConfig {
    pub server: Vec<ServerEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ServerEntry {
    pub id: String,
    pub primary_extensions: Vec<String>,
    pub secondary_extensions: Vec<String>,
    pub priority: String,
    pub andon_code_prefixes: Option<Vec<String>>,
    /// Path to the server binary. `None` means the server is not auto-spawned.
    pub command: Option<String>,
    /// Arguments passed to the server binary. Defaults to `["serve", "--stdio"]`.
    pub args: Option<Vec<String>>,
}

impl ServerEntry {
    pub fn effective_args(&self) -> Vec<String> {
        self.args
            .clone()
            .unwrap_or_else(|| vec!["serve".to_string(), "--stdio".to_string()])
    }

    /// CC-002: probe the server by attempting to spawn its command.
    /// A successful spawn (even if the process exits immediately) counts as reachable.
    /// The _timeout parameter is reserved for future async probing.
    pub fn probe(&mut self, _timeout: std::time::Duration) -> std::io::Result<()> {
        let cmd = self.command.as_deref().unwrap_or("true");
        std::process::Command::new(cmd)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|_| ())
    }
}

const DEFAULT_ANDON_PREFIXES: &[&str] = &["WASM4PM-", "ANTI-LLM-", "GGEN-"];

impl CompositorConfig {
    pub fn from_toml_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Walk from the current directory upward looking for `lsp-max.toml`.
    /// Stops (returning `None`) when it reaches a directory that contains a
    /// `Cargo.toml` with `[workspace]` and no `lsp-max.toml` was found.
    /// Collect all ANDON code prefixes across all servers, deduplicated.
    /// Servers that declare `andon_code_prefixes` use their own list; servers
    /// without the field fall back to the legacy hardcoded defaults.
    /// Per-server ANDON prefix map: server_id → prefix list.
    /// Servers without explicit `andon_code_prefixes` get the static defaults.
    /// Used by `MergeContext::from_config` to wire per-server C_D routing.
    pub fn per_server_andon_prefixes(&self) -> std::collections::HashMap<String, Vec<String>> {
        self.server
            .iter()
            .map(|s| {
                let prefixes = s.andon_code_prefixes.clone().unwrap_or_else(|| {
                    DEFAULT_ANDON_PREFIXES
                        .iter()
                        .map(|p| p.to_string())
                        .collect()
                });
                (s.id.clone(), prefixes)
            })
            .collect()
    }

    pub fn all_andon_prefixes(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut out: Vec<&str> = Vec::new();
        for server in &self.server {
            match &server.andon_code_prefixes {
                Some(v) => {
                    for p in v {
                        if seen.insert(p.as_str()) {
                            out.push(p.as_str());
                        }
                    }
                }
                None => {
                    for p in DEFAULT_ANDON_PREFIXES {
                        if seen.insert(*p) {
                            out.push(p);
                        }
                    }
                }
            }
        }
        out
    }

    pub fn load() -> Option<Self> {
        let mut dir = std::env::current_dir().ok()?;
        loop {
            let toml_path = dir.join("lsp-max.toml");
            if toml_path.exists() {
                return Self::from_toml_file(&toml_path).ok();
            }
            let cargo_toml = dir.join("Cargo.toml");
            if cargo_toml.exists() {
                let content = std::fs::read_to_string(&cargo_toml).ok()?;
                if content.contains("[workspace]") {
                    return None;
                }
            }
            dir = dir.parent()?.to_path_buf();
        }
    }

    /// Load `lsp-max.toml` (static) and `.claude/lsp-max-auto.toml` (auto-discovered),
    /// merging both. Auto-discovered servers that share an `id` with a static entry are
    /// silently dropped — static config always wins.
    pub fn load_with_auto() -> Option<Self> {
        let mut base = Self::load();
        if let Some(auto_path) = Self::find_auto_config() {
            if let Ok(auto) = Self::from_toml_file(&auto_path) {
                match base.as_mut() {
                    Some(b) => b.merge(auto),
                    None => base = Some(auto),
                }
            }
        }
        base
    }

    /// Merge `other` into `self`. Entries whose `id` already exists in `self` are dropped;
    /// first occurrence (from `self`) wins so static config is never overridden.
    pub fn merge(&mut self, other: CompositorConfig) {
        let existing: std::collections::HashSet<String> =
            self.server.iter().map(|s| s.id.clone()).collect();
        for entry in other.server {
            if !existing.contains(&entry.id) {
                self.server.push(entry);
            }
        }
    }

    fn find_auto_config() -> Option<std::path::PathBuf> {
        let mut dir = std::env::current_dir().ok()?;
        loop {
            let candidate = dir.join(".claude").join("lsp-max-auto.toml");
            if candidate.exists() {
                return Some(candidate);
            }
            let cargo_toml = dir.join("Cargo.toml");
            if cargo_toml.exists() {
                let content = std::fs::read_to_string(&cargo_toml).ok()?;
                if content.contains("[workspace]") {
                    return None;
                }
            }
            dir = dir.parent()?.to_path_buf();
        }
    }
}
