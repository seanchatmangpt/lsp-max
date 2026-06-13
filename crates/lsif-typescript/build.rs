use lsp_max_ast_codegen::generate;
use std::{fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LSIF_TS_REGEN");

    if std::env::var("LSIF_TS_REGEN").unwrap_or_default() != "1" {
        return;
    }

    let output = PathBuf::from("src/generated.rs");
    let code = generate(
        tree_sitter_typescript::TYPESCRIPT_NODE_TYPES,
        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        None,
    );
    fs::write(output, code.to_string()).unwrap();
}
