use std::fs;

#[test]
fn test_gc006_authority_surface_lock() {
    let mut lsp_max_root = std::env::current_dir().unwrap();
    while !lsp_max_root.join("Cargo.toml").exists()
        || !std::fs::read_to_string(lsp_max_root.join("Cargo.toml"))
            .unwrap()
            .contains("[workspace]")
    {
        if let Some(parent) = lsp_max_root.parent() {
            lsp_max_root = parent.to_path_buf();
        } else {
            panic!("Could not find workspace root for lsp-max");
        }
    }
    let ggen_root = lsp_max_root.parent().unwrap().join("ggen");

    // no-shadow-crates
    {
        // Ensure no forbidden shadow crates exist in the workspaces.
        let forbidden = vec![
            lsp_max_root.join("crates/wasm4pm"),
            lsp_max_root.join("crates/wasm4pm-proper"),
            lsp_max_root.join("crates/wasm4pm-compat"),
            ggen_root.join("crates/wasm4pm"),
            ggen_root.join("crates/wasm4pm-proper"),
            ggen_root.join("crates/wasm4pm-compat"),
        ];
        for path in forbidden {
            assert!(
                !path.exists(),
                "Forbidden shadow crate found: {}",
                path.display()
            );
        }
    }

    // gc-sealed-baseline
    {
        #[derive(serde::Deserialize, serde::Serialize, Clone)]
        struct BaselineManifest {
            allowed_ignored_directories: Vec<String>,
            forbidden_generated_paths: Vec<String>,
            ignored_inventory: Vec<String>,
            tracked_status: std::collections::BTreeMap<String, String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            digest: Option<String>,
        }

        let parent_dir = lsp_max_root.parent().unwrap().to_path_buf();
        let check_clean = |repo_name: &str| {
            let repo_path = parent_dir.join(repo_name);
            assert!(
                repo_path.exists(),
                "Sealed authority workspace path does not exist: {}",
                repo_path.display()
            );

            let manifest_path = repo_path.join(".gc-sealed-baseline");
            assert!(
                manifest_path.exists(),
                "Manifest .gc-sealed-baseline does not exist for {}",
                repo_name
            );

            let manifest_content = fs::read_to_string(&manifest_path).unwrap();
            let mut manifest: BaselineManifest =
                serde_json::from_str(&manifest_content).expect("Failed to parse manifest JSON");

            let expected_digest = manifest
                .digest
                .clone()
                .expect("No digest field in manifest");
            manifest.digest = None;

            let serialized = serde_json::to_string(&manifest)
                .expect("Failed to serialize manifest for digest validation");

            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(serialized.as_bytes());
            let actual_digest = format!("{:x}", hasher.finalize());

            assert_eq!(
                actual_digest, expected_digest,
                "Cryptographic digest mismatch in manifest for repo {}",
                repo_name
            );

            let output = std::process::Command::new("git")
                .args([
                    "-C",
                    repo_path.to_str().unwrap(),
                    "status",
                    "--porcelain",
                    "--ignored",
                ])
                .output()
                .unwrap_or_else(|_| panic!("Failed to run git status on {}", repo_name));
            let status_out = String::from_utf8_lossy(&output.stdout);

            for line in status_out.lines() {
                if line.is_empty() {
                    continue;
                }
                let status = &line[0..2];
                let mut path = line[3..].to_string();
                if path.starts_with('"') && path.ends_with('"') {
                    path = path[1..path.len() - 1].to_string();
                }

                if path == ".gc-sealed-baseline"
                    || path.contains(".DS_Store")
                    || path.starts_with(".claude/")
                    || path.ends_with(".swp")
                    || path.ends_with(".swo")
                {
                    continue;
                }

                for pattern in &manifest.forbidden_generated_paths {
                    if path.contains(pattern) {
                        panic!(
                            "Sealed repo {} contains forbidden path matching policy '{}': {}",
                            repo_name, pattern, path
                        );
                    }
                }

                if status == "!!" {
                    let in_inventory = manifest.ignored_inventory.contains(&path)
                        || manifest.ignored_inventory.contains(&format!("{}/", path));
                    let in_allowed_dir = manifest
                        .allowed_ignored_directories
                        .iter()
                        .any(|dir| path == *dir || path.starts_with(&format!("{}/", dir)));
                    assert!(
                        in_inventory || in_allowed_dir,
                        "New non-baselined ignored file found in sealed repo {}: {} (status: {})",
                        repo_name,
                        path,
                        status
                    );
                } else if status == "??" {
                    let allowed = manifest.ignored_inventory.contains(&path)
                        || manifest
                            .allowed_ignored_directories
                            .iter()
                            .any(|dir| path == *dir || path.starts_with(&format!("{}/", dir)));
                    assert!(
                        allowed,
                        "New untracked file found in sealed repo {}: {}",
                        repo_name, path
                    );
                } else {
                    let in_allowed_dir = manifest
                        .allowed_ignored_directories
                        .iter()
                        .any(|dir| path == *dir || path.starts_with(&format!("{}/", dir)));
                    if !in_allowed_dir {
                        assert!(
                            manifest.tracked_status.contains_key(&path),
                            "New tracked change found in sealed repo {}: {} (status: {})",
                            repo_name,
                            path,
                            status
                        );
                    }
                }
            }
        };

        check_clean("wasm4pm");
        check_clean("wasm4pm-compat");
    }

    // wasm4pm-lsp-no-local-conformance
    {
        let lsp_src =
            fs::read_to_string(lsp_max_root.join("crates/wasm4pm-lsp/src/main.rs")).unwrap();
        // The LSP must not implement its own conformance loop (e.g. searching for markers or checking cardinalities directly)
        assert!(!lsp_src.contains("check_gall_conformance(&ocel) {")); // It shouldn't implement the logic directly
        assert!(!lsp_src.contains("GallVerdict::Fit {")); // It shouldn't manufacture these
                                                          // It MUST call the adapter
        assert!(lsp_src.contains("gc005_wasm4pm_adapter::analyze_ocel"));
    }

    // adapter-no-fake-fit
    {
        let adapter_src =
            fs::read_to_string(lsp_max_root.join("crates/gc005-wasm4pm-adapter/src/lib.rs"))
                .unwrap();
        // The adapter must not contain string replacement logic for verdicts or hardcode "FIT" conditionally outside of matching the authority's enum.
        assert!(!adapter_src.contains("verdict = \"FIT\""));
        assert!(!adapter_src.contains("fitness == 1.0"));
    }

    // adapter-uses-sealed-authority
    {
        let adapter_src =
            fs::read_to_string(lsp_max_root.join("crates/gc005-wasm4pm-adapter/src/lib.rs"))
                .unwrap();
        // The adapter MUST import check_gall_conformance from wasm4pm
        assert!(adapter_src.contains("check_gall_conformance"));
        assert!(adapter_src.contains("check_gall_conformance(&ocel)"));

        let adapter_cargo =
            fs::read_to_string(lsp_max_root.join("crates/gc005-wasm4pm-adapter/Cargo.toml"))
                .unwrap();
        assert!(adapter_cargo.contains("wasm4pm = { path = \"../../../wasm4pm/wasm4pm\" }"));
        assert!(
            adapter_cargo.contains("wasm4pm-compat = { workspace = true }")
                || adapter_cargo.contains("wasm4pm-compat = { path = \"../../../wasm4pm-compat\"")
        );
    }

    // Architecture Contract Checks
    let authority_surface =
        fs::read_to_string(lsp_max_root.join("crates/playground/receipts/authority_surface.toml"))
            .unwrap_or_default();
    if !authority_surface.is_empty() {
        assert!(
            authority_surface.contains("wasm4pm::gall")
                || authority_surface.contains("wasm4pm_algos::gall")
        );
        assert!(authority_surface.contains("check_gall_conformance"));
    }
}
