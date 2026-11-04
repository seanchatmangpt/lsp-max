// Court for the ontology-first registry-init generation rule (ggen 26.10.11).
// Chicago-style: real ggen subprocess over real files; assertions on real bytes.
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use lsp_max_compositor::registry::{ExtensionRouter, SpawnRefusal};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn stage_scratch() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    (dir, root)
}

fn copy_inputs(root: &PathBuf) {
    fs::copy(
        crate_dir().join("compositor.ttl"),
        root.join("compositor.ttl"),
    )
    .unwrap();
    fs::create_dir_all(root.join("queries")).unwrap();
    fs::copy(
        crate_dir().join("queries/registry_servers.rq"),
        root.join("queries/registry_servers.rq"),
    )
    .unwrap();
    fs::copy(
        crate_dir().join("registry_init.tera"),
        root.join("registry_init.tera"),
    )
    .unwrap();
}

fn run_ggen(root: &PathBuf) -> String {
    let out = Command::new("ggen")
        .arg("sync")
        .arg("run")
        // Registry projection is content-pack sync, no SBB manufacture —
        // the disclosed FM-LAW-015 opt-out (ggen sync.rs lawful escape hatch).
        .arg("--skip-ea-admission")
        .current_dir(root)
        .output()
        .expect("spawn ggen");
    assert!(
        out.status.success(),
        "ggen sync run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::read_to_string(root.join("out.rs")).expect("generated out.rs")
}

fn court_manifest() -> &'static str {
    r#"
[project]
name = "registry-court-scratch"
version = "1.0.0"

[ontology]
source = "compositor.ttl"

[generation]
output_dir = "."

[[generation.rules]]
name        = "registry-init-from-ontology"
query       = { file = "queries/registry_servers.rq" }
template    = { file = "registry_init.tera" }
output_file = "out.rs"
mode        = "Overwrite"
skip_empty  = true
when        = "PREFIX lspmax: <https://lsp-max.dev/ontology#> ASK { ?s a lspmax:ChildServer }"
"#
}

fn stage() -> (tempfile::TempDir, PathBuf) {
    let (guard, root) = stage_scratch();
    copy_inputs(&root);
    fs::write(root.join("ggen.toml"), court_manifest()).unwrap();
    (guard, root)
}

#[test]
fn generated_registry_knows_all_four_servers() {
    let generated = fs::read_to_string(crate_dir().join("src/registry_init.rs"))
        .expect("read generated registry_init.rs");
    for (id, tier) in [
        ("claude-code-config-lsp", "ChildTier::DiagnosticsOnly"),
        ("vscode-json-lsp", "ChildTier::Primary"),
        ("markdown-lsp", "ChildTier::Primary"),
        ("justfile-lsp", "ChildTier::Primary"),
    ] {
        assert!(
            generated.contains(&format!("id: \"{id}\"")),
            "generated registry missing server id {id}"
        );
        assert!(
            generated.contains(tier),
            "generated registry missing tier token {tier}"
        );
    }
    assert!(generated.contains("new_from_manifest"));
}

#[test]
fn deleting_an_individual_drops_it_from_regeneration() {
    let (_guard, root) = stage();
    let ttl_path = root.join("compositor.ttl");
    let ttl = fs::read_to_string(&ttl_path).unwrap();
    // Delete the whole Turtle statement block for the individual: from its
    // subject line through the terminating " ." line. Line-only filtering
    // would orphan the predicate-object continuation lines (parse error).
    let mut skip_block = false;
    let mut kept: Vec<&str> = Vec::new();
    for line in ttl.lines() {
        if line.starts_with("lspmax:server-claude-code-config-lsp ") {
            skip_block = true;
            continue;
        }
        if skip_block {
            if line.trim_end().ends_with(" .") {
                skip_block = false;
            }
            continue;
        }
        kept.push(line);
    }
    let mutated = kept.join("\n");
    fs::write(&ttl_path, mutated).unwrap();
    let generated = run_ggen(&root);
    assert!(
        !generated.contains("claude-code-config-lsp"),
        "deleted individual must not appear after regeneration"
    );
    for survivor in ["vscode-json-lsp", "markdown-lsp", "justfile-lsp"] {
        assert!(
            generated.contains(&format!("id: \"{survivor}\"")),
            "survivor {survivor} must remain after deletion"
        );
    }
    assert!(generated.contains("ChildTier::Primary"));
}

#[test]
fn determinism_double_run_byte_identical() {
    let (_guard, root) = stage();
    let first = run_ggen(&root);
    let second = run_ggen(&root);
    assert_eq!(
        first, second,
        "two consecutive ggen real runs must be byte-identical"
    );
    assert!(first.contains("claude-code-config-lsp"));
}

// ---- G4 courts (LM-1): spawn-capable registry + command-less typed refusal ----

#[test]
fn spawn_capable_registry_carries_command_for_declared_server() {
    // Real manifest registry: claude-code-config-lsp declares lspmax:command
    // in compositor.ttl; the seam yields it. Others declare none.
    let router = ExtensionRouter::new_from_manifest();
    let servers = router.servers_for_command("claude-code-config-lsp");
    assert_eq!(servers.len(), 1, "exactly one server owns the command");
    assert_eq!(servers[0].id, "claude-code-config-lsp");
    let cmd = servers[0]
        .spawn_command()
        .expect("declared command must resolve");
    assert_eq!(cmd, "claude-code-config-lsp");
}

#[test]
fn command_less_server_spawn_is_typed_refusal_not_guess() {
    // ggen-lsp has no lspmax:command in the ontology — spawn attempts through
    // the seam refuse typed instead of guessing a binary name from the id.
    let router = ExtensionRouter::new_from_manifest();
    let servers = router.servers_for_filename("main.ttl");
    assert!(servers.iter().any(|s| s.id == "ggen-lsp"));
    let ggen = servers.iter().find(|s| s.id == "ggen-lsp").unwrap();
    let err = ggen.spawn_command().expect_err("must refuse");
    assert_eq!(
        err,
        SpawnRefusal {
            server_id: "ggen-lsp".to_string()
        }
    );
    assert!(err
        .to_string()
        .contains("declares no lspmax:command"));
}

#[test]
fn generated_registry_init_emits_commands_field() {
    // The generated surface (src/registry_init.rs) must carry the commands
    // field: declared server gets the command, undeclared get Vec::new().
    let generated =
        fs::read_to_string(crate_dir().join("src/registry_init.rs")).expect("read registry_init.rs");
    assert!(
        generated.contains("commands: vec![\"claude-code-config-lsp\".into()]"),
        "declared server must carry its command in generated registry"
    );
    let empty = generated.matches("commands: Vec::new()").count();
    assert_eq!(
        empty, 9,
        "command-less registrations must render commands: Vec::new() (11 registrations, 2 carry the ccc-lsp command), got {empty}"
    );
}
