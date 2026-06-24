use std::fs;
use std::path::Path;

fn walk_dir(dir: &Path, violations: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == ".git"
                || name == ".claude"
                || name == "target"
                || name == "vendors"
                || name == "scratch"
                || name == "examples"
                || name == "fixtures"
            {
                continue;
            }
            if path.is_dir() {
                walk_dir(&path, violations);
            } else if path.is_file()
                && (name == "Cargo.toml" || name == "pack.toml" || name == "sync_target.rs")
            {
                if let Ok(content) = fs::read_to_string(&path) {
                    for (i, line) in content.lines().enumerate() {
                        if line.contains("1.0.0") || line.contains("v1.0.0") {
                            violations.push(format!(
                                "Violation in {:?} at line {}: {}",
                                path,
                                i + 1,
                                line.trim()
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "requires ggen sibling repo to be fully on CalVer — BLOCKED until ggen crates migrated from 1.0.0"]
fn test_gc006_release_law_calver_lock() {
    let current_dir = std::env::current_dir().unwrap();
    let lsp_max_root = current_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let ggen_root = lsp_max_root.parent().unwrap().join("ggen");

    let mut lsp_violations = Vec::new();
    let mut ggen_violations = Vec::new();

    walk_dir(&lsp_max_root, &mut lsp_violations);
    walk_dir(&ggen_root, &mut ggen_violations);

    if !lsp_violations.is_empty() {
        panic!(
            "RELEASE_LAW_CALVER_LOCK violated in lsp-max. Found forbidden version 1.0.0:\n{}",
            lsp_violations.join("\n")
        );
    }

    if !ggen_violations.is_empty() {
        println!(
            "RELEASE_LAW_CALVER_LOCK: ggen has pending migrations (ignored by default):\n{}",
            ggen_violations.join("\n")
        );
    }
}
