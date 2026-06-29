#[derive(Debug, serde::Deserialize, Default)]
pub struct AutoScanConfig {
    /// When false, auto-discovery (`lsp-max-auto.toml`) is skipped entirely.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Shell hook invoked before loading the auto config so discovery scripts
    /// can regenerate `.claude/lsp-max-auto.toml`.
    pub run_hook: Option<String>,
    /// Resolution policy when the same server `id` appears in both configs.
    #[serde(default)]
    pub dedup_strategy: DedupStrategy,
    /// Milliseconds to wait when probing a server command via `ServerEntry::probe`.
    #[serde(default = "default_probe_timeout_ms")]
    pub probe_timeout_ms: u64,
}

fn default_true() -> bool {
    true
}

fn default_probe_timeout_ms() -> u64 {
    500
}

/// Resolution policy when the same server `id` appears in both static and auto config.
#[derive(Debug, serde::Deserialize, Default, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DedupStrategy {
    /// Static `lsp-max.toml` entry wins; auto entry is silently dropped.
    #[default]
    StaticWins,
    /// Auto-discovered entry wins; static entry is replaced.
    AutoWins,
    /// Conflict emits a log warning; static entry is retained.
    ErrorOnConflict,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct CompositorConfig {
    #[serde(default)]
    pub server: Vec<ServerEntry>,
    #[serde(default)]
    pub auto_scan: AutoScanConfig,
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

    /// Probe the server by spawning its command with `--version`.
    ///
    /// Returns `Ok(())` when the binary spawns and exits within `timeout` (any exit
    /// code is accepted — we only verify the binary is present and executable).
    /// Returns `Err(NotFound)` when no `command` is configured, or when the OS cannot
    /// find the binary. Returns `Err(TimedOut)` when the child does not exit in time.
    pub fn probe(&self, timeout: std::time::Duration) -> std::io::Result<()> {
        let command = match &self.command {
            Some(c) => c.clone(),
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no command configured for server entry",
                ));
            }
        };

        let mut child = std::process::Command::new(&command)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        // Enforce timeout via a background thread; the spawn cost is negligible for
        // probe calls (one-shot at config load time, not on the hot path).
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait());
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(_status)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("probe of '{}' timed out after {:?}", command, timeout),
            )),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "probe thread disconnected unexpectedly",
            )),
        }
    }

    /// Priority rank: lower = higher precedence (Primary wins over Secondary, etc.).
    fn priority_rank(&self) -> u8 {
        match self.priority.to_lowercase().as_str() {
            "primary" => 0,
            "secondary" => 1,
            "diagnostics_only" | "diagnosticsonly" | "diagnostics-only" => 2,
            _ => 3,
        }
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
    /// merging both according to `[auto_scan]` settings.
    ///
    /// When `auto_scan.run_hook` is set and `auto_scan.enabled` is true, the hook is
    /// executed via the shell before loading the auto config so discovery scripts can
    /// regenerate the file.
    ///
    /// Dedup is by `id`. The default strategy (`static-wins`) retains the static entry
    /// on collision. When two entries share the same `id`, `command`, and
    /// `primary_extensions` but differ only in `priority`, the higher-priority entry wins.
    pub fn load_with_auto() -> Option<Self> {
        let mut base = Self::load();

        let auto_scan_enabled = base
            .as_ref()
            .map(|b| b.auto_scan.enabled)
            .unwrap_or(true);

        if !auto_scan_enabled {
            return base;
        }

        if let Some(hook) = base.as_ref().and_then(|b| b.auto_scan.run_hook.as_deref()) {
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(hook)
                .status();
        }

        if let Some(auto_path) = Self::find_auto_config() {
            if let Ok(auto) = Self::from_toml_file(&auto_path) {
                let strategy = base
                    .as_ref()
                    .map(|b| b.auto_scan.dedup_strategy.clone())
                    .unwrap_or_default();
                match base.as_mut() {
                    Some(b) => b.merge_with_strategy(auto, &strategy),
                    None => base = Some(auto),
                }
            }
        }
        base
    }

    /// Merge `other` into `self` using the default `static-wins` strategy.
    pub fn merge(&mut self, other: CompositorConfig) {
        self.merge_with_strategy(other, &DedupStrategy::StaticWins);
    }

    /// Merge `other` into `self` using the specified dedup strategy.
    ///
    /// - `StaticWins` — auto entry dropped on `id` collision, unless it has higher
    ///   priority for the same `command` + `primary_extensions`, in which case the
    ///   priority field is upgraded in place.
    /// - `AutoWins` — auto entry replaces the static entry on collision.
    /// - `ErrorOnConflict` — logs a warning and retains the static entry.
    pub fn merge_with_strategy(&mut self, other: CompositorConfig, strategy: &DedupStrategy) {
        let mut id_to_idx: std::collections::HashMap<String, usize> = self
            .server
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.clone(), i))
            .collect();

        for entry in other.server {
            if let Some(&idx) = id_to_idx.get(&entry.id) {
                match strategy {
                    DedupStrategy::StaticWins => {
                        // Upgrade priority in place when same command + extensions but
                        // the auto entry carries a higher-priority tier declaration.
                        let existing = &self.server[idx];
                        let same_command = existing.command == entry.command;
                        let same_ext = existing.primary_extensions == entry.primary_extensions;
                        if same_command && same_ext
                            && entry.priority_rank() < existing.priority_rank()
                        {
                            self.server[idx] = entry;
                        }
                    }
                    DedupStrategy::AutoWins => {
                        self.server[idx] = entry;
                    }
                    DedupStrategy::ErrorOnConflict => {
                        tracing::warn!(
                            id = %entry.id,
                            "COMPOSITOR-CONFLICT: server id '{}' present in both static and \
                             auto config; static entry retained",
                            entry.id
                        );
                    }
                }
            } else {
                let new_idx = self.server.len();
                id_to_idx.insert(entry.id.clone(), new_idx);
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
