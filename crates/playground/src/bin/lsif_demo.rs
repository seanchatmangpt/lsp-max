use std::io::{self};
use tower_lsp_max::lsp_types::{MarkupContent, MarkupKind, Position, Range, SymbolKind};
use tower_lsp_max_lsif::lsif::{HoverContents, RangeTag, ToolInfo};
use tower_lsp_max_lsif::lsif_builder::LsifBuilder;

fn main() {
    let stdout = io::stdout();
    let mut builder = LsifBuilder::new(stdout.lock());

    // 1. MetaData
    builder
        .emit_metadata(
            "0.6.0",
            "file:///playground",
            ToolInfo {
                name: "tower-lsp-max-lsif-demo".to_string(),
                version: Some("0.1.0".to_string()),
                args: None,
            },
        )
        .unwrap();

    // 2. Project
    let project_id = builder
        .emit_project(Some("rust"), Some("file:///playground".to_string()))
        .unwrap();

    // 3. Document
    let doc_id = builder
        .emit_document("file:///playground/src/main.rs", "rust")
        .unwrap();

    // 4. Range (a symbol in the document)
    let range = Range {
        start: Position {
            line: 0,
            character: 4,
        },
        end: Position {
            line: 0,
            character: 8,
        }, // e.g., "main"
    };
    let tag = RangeTag::Definition {
        text: "main".to_string(),
        kind: SymbolKind::FUNCTION,
        full_range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 2,
                character: 1,
            },
        },
        detail: Some("fn main()".to_string()),
    };
    let range_id = builder
        .emit_range(range.start, range.end, Some(tag))
        .unwrap();

    // 5. ResultSet (Hub for navigation)
    let result_set_id = builder.emit_result_set().unwrap();

    // Link Range to ResultSet
    builder
        .bind_next(range_id.clone(), result_set_id.clone())
        .unwrap();

    // 6. HoverResult
    let hover_contents = HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: "```rust\nfn main()\n```\nThe entry point of the program.".to_string(),
    });
    builder
        .bind_hover(result_set_id.clone(), hover_contents)
        .unwrap();

    // 7. DefinitionResult
    builder
        .bind_definition(
            result_set_id.clone(),
            vec![range_id.clone()],
            doc_id.clone(),
        )
        .unwrap();

    // Lifecycle Ends
    builder.end_document(doc_id).unwrap();
    builder.end_project(project_id).unwrap();
}
