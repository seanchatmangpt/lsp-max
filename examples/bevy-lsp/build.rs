fn main() {
    let source = tree_sitter_rust::NODE_TYPES;
    let tokens = std::collections::HashMap::from([("`", "Backtick")]);

    let token_stream =
        auto_lsp_codegen::generate(source, &tree_sitter_rust::LANGUAGE.into(), Some(tokens));

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("generated_ast.rs");

    std::fs::write(&dest_path, token_stream.to_string()).expect("Failed to write AST generation");

    println!("cargo:rerun-if-changed=node-types.json");
    println!("cargo:rerun-if-changed=build.rs");
}
