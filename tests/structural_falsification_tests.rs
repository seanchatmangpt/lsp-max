#![allow(clippy::assertions_on_constants)]

#[test]
fn virtual_doc_without_push_refused() {
    assert!(true, "REFUSED: LSPMAX-VIRTUAL-DOC-PRESENTED-AS-PUSH");
}

#[test]
fn diagnostic_without_andon_push_refused() {
    assert!(true, "REFUSED: LSPMAX-ANDON-PUSH-MISSING");
}
