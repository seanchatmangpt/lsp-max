use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub template_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackListResult {
    pub packs: Vec<PackInfo>,
    pub packs_dir: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackValidateResult {
    pub pack_name: String,
    pub valid: bool,
    pub issues: Vec<String>,
    pub template_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackInitResult {
    pub name: String,
    pub dir: String,
    pub created: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackInfoResult {
    pub info: PackInfo,
    pub templates: Vec<String>,
    pub queries: Vec<String>,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct PackService;

impl PackService {
    pub fn new() -> Self {
        Self
    }

    fn templates_root(base: &str) -> PathBuf {
        PathBuf::from(base).join("templates")
    }

    fn discover_packs(base: &str) -> Vec<PackInfo> {
        let root = Self::templates_root(base);
        let mut packs = vec![];
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let template_count = fs::read_dir(&path)
                        .map(|e| {
                            e.flatten()
                                .filter(|f| {
                                    f.path().extension().and_then(|x| x.to_str()) == Some("tera")
                                })
                                .count()
                        })
                        .unwrap_or(0);
                    packs.push(PackInfo {
                        name,
                        version: "0.1.0".into(),
                        description: "ggen template pack — CANDIDATE".into(),
                        template_count,
                        status: "CANDIDATE".into(),
                    });
                }
            }
        }
        packs
    }

    pub fn list(&self, base: &str) -> PackListResult {
        let packs = Self::discover_packs(base);
        PackListResult {
            packs_dir: Self::templates_root(base).display().to_string(),
            packs,
            status: "CANDIDATE".into(),
        }
    }

    pub fn info(&self, name: &str, base: &str) -> Result<PackInfoResult> {
        let pack_dir = Self::templates_root(base).join(name);
        if !pack_dir.exists() {
            return Err(clap_noun_verb::error::NounVerbError::execution_error(
                format!(
                    "pack '{name}' not found in {}",
                    Self::templates_root(base).display()
                ),
            ));
        }
        let mut templates = vec![];
        let mut queries: Vec<String> = vec![];
        if let Ok(entries) = fs::read_dir(&pack_dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.ends_with(".tera") {
                    templates.push(fname);
                }
            }
        }
        // check queries/ subdirectory alongside templates root
        let query_dir = PathBuf::from(base).join("queries").join(name);
        if let Ok(entries) = fs::read_dir(&query_dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.ends_with(".sparql") {
                    queries.push(fname);
                }
            }
        }
        templates.sort();
        queries.sort();
        Ok(PackInfoResult {
            info: PackInfo {
                name: name.to_string(),
                version: "0.1.0".into(),
                description: format!("ggen template pack: {name} — CANDIDATE"),
                template_count: templates.len(),
                status: "CANDIDATE".into(),
            },
            templates,
            queries,
            status: "CANDIDATE".into(),
        })
    }

    pub fn validate(&self, name: &str, base: &str) -> PackValidateResult {
        let pack_dir = Self::templates_root(base).join(name);
        let mut issues = vec![];
        let mut template_count = 0;

        if !pack_dir.exists() {
            issues.push(format!(
                "OPEN: pack directory not found: {}",
                pack_dir.display()
            ));
            return PackValidateResult {
                pack_name: name.to_string(),
                valid: false,
                issues,
                template_count,
                status: "OPEN".into(),
            };
        }

        if let Ok(entries) = fs::read_dir(&pack_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("tera") {
                    template_count += 1;
                    // law check: no tower-lsp references in templates
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if content.contains("tower-lsp") || content.contains("tower_lsp") {
                            issues.push(format!(
                                "LawViolation: {} contains forbidden tower-lsp reference",
                                entry.file_name().to_string_lossy()
                            ));
                        }
                    }
                }
            }
        }

        PackValidateResult {
            pack_name: name.to_string(),
            valid: issues.is_empty(),
            issues,
            template_count,
            status: "CANDIDATE".into(),
        }
    }

    pub fn init(&self, name: &str, base: &str) -> Result<PackInitResult> {
        let pack_dir = Self::templates_root(base).join(name);
        fs::create_dir_all(&pack_dir)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

        let pack_toml = format!(
            "[pack]\nname = \"{name}\"\nversion = \"0.1.0\"\ndescription = \"ggen template pack — CANDIDATE\"\n"
        );
        let toml_path = pack_dir.join("pack.toml");
        if !toml_path.exists() {
            fs::write(&toml_path, pack_toml).map_err(|e| {
                clap_noun_verb::error::NounVerbError::execution_error(e.to_string())
            })?;
        }

        let readme_path = pack_dir.join("README.md");
        if !readme_path.exists() {
            fs::write(
                &readme_path,
                format!("# {name} ggen pack\n\nStatus: CANDIDATE\n"),
            )
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
        }

        Ok(PackInitResult {
            name: name.to_string(),
            dir: pack_dir.display().to_string(),
            created: vec!["pack.toml".into(), "README.md".into()],
            status: "CANDIDATE".into(),
        })
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[verb("list")]
pub fn list(dir: Option<String>) -> Result<PackListResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(PackService::new().list(&dir))
}

#[verb("info")]
pub fn info(name: String, dir: Option<String>) -> Result<PackInfoResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    PackService::new().info(&name, &dir)
}

#[verb("validate")]
pub fn validate(name: String, dir: Option<String>) -> Result<PackValidateResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(PackService::new().validate(&name, &dir))
}

#[verb("init")]
pub fn init(name: String, dir: Option<String>) -> Result<PackInitResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    PackService::new().init(&name, &dir)
}
