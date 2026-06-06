//! Functions to apply workspace edits and text modifications.

/// Applies a list of changes defined in a `WorkspaceEdit` to files in the local filesystem.
///
/// Returns `Ok(())` on success, or an `Err` describing the failure.
pub fn apply_workspace_edit(edit: &lsp_types_max::WorkspaceEdit) -> Result<(), String> {
    if let Some(changes) = &edit.changes {
        for (url, edits) in changes {
            let parsed_url = url::Url::parse(url.as_str())
                .map_err(|e| format!("Invalid URL '{}': {}", url.as_str(), e))?;
            if parsed_url.scheme() != "file" {
                return Err(format!("Unsupported URL scheme: {}", parsed_url.scheme()));
            }
            let path = parsed_url
                .to_file_path()
                .map_err(|_| format!("Invalid file path for URL: {}", url.as_str()))?;

            let mut content = if path.exists() {
                std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?
            } else {
                String::new()
            };

            let mut sorted_edits = edits.clone();
            sorted_edits.sort_by(|a, b| {
                let start_a = a.range.start;
                let start_b = b.range.start;
                if start_a.line != start_b.line {
                    start_b.line.cmp(&start_a.line)
                } else {
                    start_b.character.cmp(&start_a.character)
                }
            });

            for text_edit in sorted_edits {
                content = apply_text_edit(&content, &text_edit)?;
            }

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }
            std::fs::write(&path, &content)
                .map_err(|e| format!("Failed to write file {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

fn apply_text_edit(content: &str, edit: &lsp_types_max::TextEdit) -> Result<String, String> {
    let lines: Vec<&str> = content.split('\n').collect();

    let start_line = edit.range.start.line as usize;
    let start_char = edit.range.start.character as usize;
    let end_line = edit.range.end.line as usize;
    let end_char = edit.range.end.character as usize;

    let get_char_offset = |line_idx: usize, char_idx: usize| -> Result<usize, String> {
        if line_idx > lines.len() {
            return Err(format!("Line index {} out of bounds", line_idx));
        }

        let mut byte_offset = 0;
        for line in lines.iter().take(line_idx) {
            byte_offset += line.len() + 1;
        }

        if line_idx < lines.len() {
            let line_chars: Vec<char> = lines[line_idx].chars().collect();
            if char_idx > line_chars.len() {
                return Err(format!(
                    "Character index {} out of bounds for line {}",
                    char_idx, line_idx
                ));
            }
            let char_byte_len: usize = line_chars[0..char_idx].iter().map(|c| c.len_utf8()).sum();
            byte_offset += char_byte_len;
        }

        Ok(byte_offset)
    };

    let start_offset = get_char_offset(start_line, start_char)?;
    let end_offset = get_char_offset(end_line, end_char)?;

    if start_offset > end_offset || end_offset > content.len() {
        return Err("Invalid range for text edit".to_string());
    }

    let mut new_content = content[0..start_offset].to_string();
    new_content.push_str(&edit.new_text);
    new_content.push_str(&content[end_offset..]);

    Ok(new_content)
}
