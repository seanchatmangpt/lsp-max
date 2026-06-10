#[test]
fn test_codegen_dry_run() {
    let source = r#"[{"type": "identifier", "named": true}]"#;
    let _result = crate::generate(source, &tree_sitter_html::LANGUAGE.into(), None);
    assert!(!_result.is_empty());
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_python() {
    let _result = crate::generate(
        tree_sitter_python::NODE_TYPES,
        &tree_sitter_python::LANGUAGE.into(),
        None,
    );
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_html() {
    let _result = crate::generate(
        tree_sitter_html::NODE_TYPES,
        &tree_sitter_html::LANGUAGE.into(),
        None,
    );
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_javascript() {
    let _result = crate::generate(
        tree_sitter_javascript::NODE_TYPES,
        &tree_sitter_javascript::LANGUAGE.into(),
        Some(std::collections::HashMap::from([("`", "Backtick")])),
    );
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_typescript() {
    let _result = crate::generate(
        tree_sitter_typescript::TYPESCRIPT_NODE_TYPES,
        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Some(std::collections::HashMap::from([("`", "Backtick")])),
    );
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_tsx() {
    let _result = crate::generate(
        tree_sitter_typescript::TSX_NODE_TYPES,
        &tree_sitter_typescript::LANGUAGE_TSX.into(),
        Some(std::collections::HashMap::from([("`", "Backtick")])),
    );
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_c() {
    let _result = crate::generate(
        tree_sitter_c::NODE_TYPES,
        &tree_sitter_c::LANGUAGE.into(),
        Some(std::collections::HashMap::from([("\n", "Newline")])),
    );
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_c_sharp() {
    let _result = crate::generate(
        tree_sitter_c_sharp::NODE_TYPES,
        &tree_sitter_c_sharp::LANGUAGE.into(),
        None,
    );
}

#[test]
#[ignore = "long running pre-publish test"]
fn gen_haskell() {
    let _result = crate::generate(
        tree_sitter_haskell::NODE_TYPES,
        &tree_sitter_haskell::LANGUAGE.into(),
        Some(std::collections::HashMap::from([
            ("`", "Backtick"),
            ("←", "LeftArrow"),
            ("→", "RightArrow"),
            ("⇒", "DoubleRightArrow"),
            ("⊸", "RightTack"),
            ("∀", "Forall"),
            ("★", "Star"),
            ("∃", "Exists"),
            ("∷", "ColonColon"),
            ("⟦", "LeftDoubleBracket"),
            ("⟧", "RightDoubleBracket"),
        ])),
    );
}
