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
                    return None; // reached workspace root, no lsp-max.toml found
                }
            }
            dir = dir.parent()?.to_path_buf();
        }
    }
}
