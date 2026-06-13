use lsp_max_ast_codegen::generate;
use std::{fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LSIF_RUST_REGEN");

    if std::env::var("LSIF_RUST_REGEN").unwrap_or_default() != "1" {
        return;
    }

    let output = PathBuf::from("src/generated.rs");
    let code = generate(
        tree_sitter_rust::NODE_TYPES,
        &tree_sitter_rust::LANGUAGE.into(),
        None,
    );
    fs::write(output, code.to_string()).unwrap();
}
