use crate::jsonrpc::{Error, Result};
use lsp_types_max::{Hover, HoverParams, HoverContents, MarkupContent, MarkupKind};
use url::Url;
use std::process::Command;

/// Asks the server for hover information of a symbol.
pub async fn hover(params: HoverParams) -> Result<Option<Hover>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = crate::runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    // --- Vector 2: LSP-Affidavit Hover ---
    // Visually decode the 7-stage verification status directly in the editor
    // when hovering over affidavit receipt.json files.
    if url.path().ends_with("receipt.json") {
        if let Ok(path) = url.to_file_path() {
            if let Ok(output) = Command::new("affi")
                .args(["receipt", "verify", &path.to_string_lossy()])
                .output()
            {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                
                let status_icon = if output.status.success() { "✅ **ACCEPT**" } else { "❌ **REJECT**" };
                let mut hover_text = format!("### Affidavit 7-Stage Verification\n\n**Verdict:** {}\n\n", status_icon);
                
                if !stdout_str.is_empty() {
                    let mut clean_out = String::new();
                    let mut in_esc = false;
                    for c in stdout_str.chars() {
                        if c == '\x1B' { in_esc = true; continue; }
                        if in_esc { if c.is_ascii_alphabetic() { in_esc = false; } continue; }
                        clean_out.push(c);
                    }
                    hover_text.push_str("```text\n");
                    hover_text.push_str(clean_out.trim());
                    hover_text.push_str("\n```\n");
                }
                
                if !stderr_str.is_empty() {
                    let mut clean_err = String::new();
                    let mut in_esc = false;
                    for c in stderr_str.chars() {
                        if c == '\x1B' { in_esc = true; continue; }
                        if in_esc { if c.is_ascii_alphabetic() { in_esc = false; } continue; }
                        clean_err.push(c);
                    }
                    hover_text.push_str("```text\n");
                    hover_text.push_str(clean_err.trim());
                    hover_text.push_str("\n```\n");
                }

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_text,
                    }),
                    range: None,
                }));
            }
        }
    }

    if let Some(h) = crate::runtime::control_plane::views::lookup_hover(views, &url, pos) {
        Ok(Some(h))
    } else {
        Ok(None)
    }
}
