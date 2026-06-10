use std::fs;
use walkdir::WalkDir;

#[test]
fn test_no_bullshit_stubs_or_comments() {
    let forbidden_phrases = [
        "In a full implementation",
        "In a real implementation",
        "In a production",
        "In a complete",
        "unimplemented!(",
        "todo!(",
    ];

    let mut found_bullshit = false;

    for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if let Some(ext) = path.extension() {
            if ext == "rs" {
                let path_str = path.to_str().unwrap_or_default();

                // Skip target directories, state directories, and the test file itself
                if path_str.contains("/target/")
                    || path_str.contains("/target_lsp/")
                    || path_str.contains("/.claude/")
                    || path_str.contains("/.agents/")
                    || path_str.ends_with("test_no_bullshit_stubs.rs")
                {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(path) {
                    for (line_idx, line) in content.lines().enumerate() {
                        // Skip if the line explicitly wraps the stub in quotes (used in tests/snippets)
                        if line.contains("\"todo!()\"")
                            || line.contains("\"unimplemented!()\"")
                            || line.contains("\"fn main() { unimplemented!() }\"")
                            || line.contains("fn foo() { todo!() }")
                        {
                            continue;
                        }

                        let line_lower = line.to_lowercase();
                        for phrase in forbidden_phrases.iter() {
                            if line_lower.contains(&phrase.to_lowercase()) {
                                println!(
                                    "Bullshit stub detected in {}:{}: '{}'",
                                    path_str,
                                    line_idx + 1,
                                    line.trim()
                                );
                                found_bullshit = true;
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(!found_bullshit, "Codebase contains lazy stubs, 'In a...' comments, or fake implementations. Write real code.");
}
