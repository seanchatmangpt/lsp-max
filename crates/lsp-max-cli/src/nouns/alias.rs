use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasEntry {
    pub name: String,
    pub noun: String,
    pub verb: String,
    pub args: Vec<String>,
    pub created_at: String, // Unix epoch seconds
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AliasStore {
    pub aliases: HashMap<String, AliasEntry>,
}

/// Nouns that aliases may not shadow.
const RESERVED_NOUNS: &[&str] = &[
    "admission",
    "agent",
    "alias",
    "client",
    "config",
    "conformance",
    "diagnostics",
    "doctor",
    "event",
    "gate",
    "hook",
    "metamodel",
    "plugin",
    "receipt",
    "repair",
    "rpc",
    "server",
    "snapshot",
    "state",
    "telemetry",
    "workspace",
];

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct AliasService;

impl AliasService {
    pub fn new() -> Self {
        Self
    }

    fn alias_path(&self) -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{home}/.lsp-max-aliases.json")
    }

    fn load(&self) -> AliasStore {
        let path = self.alias_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self, store: &AliasStore) -> std::result::Result<(), String> {
        let path = self.alias_path();
        let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn list(&self) -> Vec<AliasEntry> {
        let store = self.load();
        let mut entries: Vec<AliasEntry> = store.aliases.into_values().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    pub fn set(
        &self,
        name: &str,
        noun: &str,
        verb: &str,
        args: Vec<String>,
    ) -> std::result::Result<(AliasEntry, bool), String> {
        if RESERVED_NOUNS.contains(&name) {
            return Err(format!("ALIAS_RESERVED: '{name}' conflicts with a built-in noun"));
        }
        let mut store = self.load();
        let replaced = store.aliases.contains_key(name);
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        let entry = AliasEntry {
            name: name.to_string(),
            noun: noun.to_string(),
            verb: verb.to_string(),
            args,
            created_at,
        };
        store.aliases.insert(name.to_string(), entry.clone());
        self.save(&store)?;
        Ok((entry, replaced))
    }

    pub fn remove(&self, name: &str) -> std::result::Result<bool, String> {
        let mut store = self.load();
        let was_present = store.aliases.remove(name).is_some();
        if was_present {
            self.save(&store)?;
        }
        Ok(was_present)
    }

    pub fn resolve(&self, name: &str) -> Option<AliasEntry> {
        let store = self.load();
        store.aliases.get(name).cloned()
    }
}

impl Default for AliasService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Debug, Serialize)]
pub struct AliasListResult {
    pub aliases: Vec<AliasEntry>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct AliasSetResult {
    pub name: String,
    pub entry: AliasEntry,
    pub replaced: bool,
}

#[derive(Debug, Serialize)]
pub struct AliasRemoveResult {
    pub name: String,
    pub was_present: bool,
}

#[derive(Debug, Serialize)]
pub struct AliasResolveResult {
    pub name: String,
    pub noun: String,
    pub verb: String,
    pub args: Vec<String>,
    pub command: String,
}

/// List all defined aliases.
#[verb("list")]
pub fn list() -> Result<AliasListResult> {
    let svc = AliasService::new();
    let aliases = svc.list();
    let count = aliases.len();
    Ok(AliasListResult { aliases, count })
}

/// Define or replace an alias mapping a short name to a noun verb expansion.
#[verb("set")]
pub fn set(
    name: String,
    noun: String,
    verb: String,
    args: Option<String>,
) -> Result<AliasSetResult> {
    let svc = AliasService::new();
    let arg_vec: Vec<String> = args
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let (entry, replaced) = svc
        .set(&name, &noun, &verb, arg_vec)
        .map_err(NounVerbError::execution_error)?;
    Ok(AliasSetResult {
        name,
        entry,
        replaced,
    })
}

/// Remove an alias by name.
#[verb("remove")]
pub fn remove(name: String) -> Result<AliasRemoveResult> {
    let svc = AliasService::new();
    let was_present = svc
        .remove(&name)
        .map_err(NounVerbError::execution_error)?;
    Ok(AliasRemoveResult { name, was_present })
}

/// Resolve an alias to its noun/verb/args expansion.
#[verb("resolve")]
pub fn resolve(name: String) -> Result<AliasResolveResult> {
    let svc = AliasService::new();
    let entry = svc
        .resolve(&name)
        .ok_or_else(|| NounVerbError::execution_error(format!("ALIAS_NOT_FOUND: {name}")))?;
    let args_joined = entry.args.join(" ");
    let command = if args_joined.is_empty() {
        format!("lsp-max-cli {} {}", entry.noun, entry.verb)
    } else {
        format!("lsp-max-cli {} {} {}", entry.noun, entry.verb, args_joined)
    };
    Ok(AliasResolveResult {
        name,
        noun: entry.noun,
        verb: entry.verb,
        args: entry.args,
        command,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service_with_env(tmp: &tempfile::TempDir) -> AliasService {
        // Set HOME to tmp so alias_path resolves inside tmp.
        std::env::set_var("HOME", tmp.path());
        AliasService::new()
    }

    #[test]
    fn list_returns_empty_when_file_absent() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        let entries = svc.list();
        assert!(entries.is_empty());
    }

    #[test]
    fn set_and_list_roundtrip() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        let (entry, replaced) = svc
            .set("chk", "gate", "check", vec![])
            .unwrap();
        assert!(!replaced);
        assert_eq!(entry.name, "chk");
        assert_eq!(entry.noun, "gate");
        assert_eq!(entry.verb, "check");
        let entries = svc.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "chk");
    }

    #[test]
    fn set_replace_marks_replaced_true() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        svc.set("chk", "gate", "check", vec![]).unwrap();
        let (_, replaced) = svc
            .set("chk", "gate", "check", vec!["--verbose".to_string()])
            .unwrap();
        assert!(replaced);
    }

    #[test]
    fn set_reserved_name_returns_err() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        let result = svc.set("gate", "gate", "check", vec![]);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("ALIAS_RESERVED"));
    }

    #[test]
    fn remove_present_alias_returns_was_present_true() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        svc.set("chk", "gate", "check", vec![]).unwrap();
        let was_present = svc.remove("chk").unwrap();
        assert!(was_present);
        assert!(svc.list().is_empty());
    }

    #[test]
    fn remove_absent_alias_returns_was_present_false() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        let was_present = svc.remove("no-such").unwrap();
        assert!(!was_present);
    }

    #[test]
    fn resolve_found_alias_returns_entry() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        svc.set("chk", "gate", "check", vec![]).unwrap();
        let entry = svc.resolve("chk").unwrap();
        assert_eq!(entry.noun, "gate");
        assert_eq!(entry.verb, "check");
    }

    #[test]
    fn resolve_absent_alias_returns_none() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        assert!(svc.resolve("no-such").is_none());
    }

    #[test]
    fn resolve_command_includes_args_when_present() {
        let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let svc = make_service_with_env(&tmp);
        svc.set("snap", "snapshot", "take", vec!["my-inst".to_string()])
            .unwrap();
        let entry = svc.resolve("snap").unwrap();
        assert_eq!(entry.args, vec!["my-inst"]);
    }
}
