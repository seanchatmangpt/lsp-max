use crate::observations::Observation;

pub fn parse_cargo_toml(filepath: &str, content: &str) -> Vec<Observation> {
    let mut obs = Vec::new();

    // Simple line-based scanning for Cargo dependencies and versions
    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for tower-lsp dependency
        if (trimmed.contains("tower-lsp") || trimmed.contains("tower_lsp"))
            && !(trimmed.contains("tower-lsp-max") || trimmed.contains("tower_lsp_max"))
        {
            obs.push(Observation {
                file_path: filepath.to_string(),
                start_byte: 0,
                end_byte: 0,
                line: line_idx + 1,
                column: 1,
                kind: "cargo_toml".to_string(),
                construct: "tower-lsp dependency".to_string(),
                context: trimmed.to_string(),
                message: "Plain tower-lsp found in Cargo dependency declaration".to_string(),
            });
        }

        // Check for version = "1.0.0" or v1.0.0
        if trimmed.replace(" ", "").contains("version=\"1.0.0\"") {
            obs.push(Observation {
                file_path: filepath.to_string(),
                start_byte: 0,
                end_byte: 0,
                line: line_idx + 1,
                column: 1,
                kind: "cargo_toml".to_string(),
                construct: "version = \"1.0.0\"".to_string(),
                context: trimmed.to_string(),
                message: "Default template version '1.0.0' found".to_string(),
            });
        }
    }

    obs
}
