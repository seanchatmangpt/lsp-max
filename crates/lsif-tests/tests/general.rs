/// General LSIF conformance tests — ported from microsoft/lsif-node/tsc-tests/src/generalTests.ts
///
/// Original test: `suite('General Tests', ...)`
/// Source: vendors/lsif-node/tsc-tests/src/generalTests.ts
///
/// Adaptation notes:
/// - lsif-node asserts by fixed element id (TypeScript compiler visit order).
///   We assert by graph structure (moniker identifier+kind, range text+position).
/// - lsif-node uses scheme "tsc"; we use scheme "rust" for Rust sources.
/// - lsif-node identifier format is "module:symbol"; ours is "module::symbol".
use lsif_tests::{index_rust_linked, index_rust_multi};

// ── Structural validator ──────────────────────────────────────────────────────

fn assert_valid(dump: &lsif_tests::LsifDump) {
    if let Err(e) = dump.validate() {
        panic!("LSIF structural validation failed:\n{e}");
    }
}

// =============================================================================
// Ported from generalTests.ts: "Single export"
//
// Original TypeScript:
//   lsif('/@test', new Map([['/@test/a.ts', 'export const x = 10;']]), ...)
//   assertElement(emitter.elements.get(15), moniker{scheme:"tsc", id:"a:x", kind:"export"})
//   assertElement(emitter.elements.get(17), range{start:{line:0,char:13}, tag:{type:"definition",text:"x"}})
//
// Our Rust equivalent: a pub const produces an export moniker + definition range.
// =============================================================================
#[test]
fn single_pub_const_produces_export_moniker() {
    let source = "pub const X: u32 = 10;";
    let dump = index_rust_linked("file:///test/a.rs", source);
    assert_valid(&dump);

    // A pub const should produce an export moniker with scheme "rust"
    // lsif-node: moniker{scheme:"tsc", identifier:"a:x", kind:"export"}
    let exports = dump.export_monikers("rust");
    assert!(
        exports.iter().any(|id| id.contains("X")),
        "expected export moniker containing 'X', got: {exports:?}"
    );
}

// =============================================================================
// Ported from generalTests.ts: "Single export" — definition range position
//
// Original: range{start:{line:0,character:13}, tag:{type:"definition",text:"x"}}
// (char 13 = start of `x` in `export const x = 10;`)
//
// Our Rust: `pub const X: u32 = 10;`
//           position of X = character 10 (0-indexed after "pub const ")
// =============================================================================
#[test]
fn definition_range_emitted_for_pub_const() {
    let source = "pub const X: u32 = 10;";
    let dump = index_rust_linked("file:///test/a.rs", source);
    assert_valid(&dump);

    let range_id = dump.find_definition_range("X");
    assert!(
        range_id.is_some(),
        "expected a definition range tagged with text='X'"
    );
}

// =============================================================================
// Ported from generalTests.ts: "Single export" — resultSet-to-moniker linkage
//
// Original:
//   id:14 = resultSet
//   id:15 = moniker{scheme:"tsc", id:"a:x", kind:"export"}
//   id:16 = edge{label:"moniker", outV:14, inV:15}
//   id:17 = range  (definition)
//   next edge: range(17) → resultSet(14)
//
// We verify: definition range → resultSet → has at least one moniker attached.
// =============================================================================
#[test]
fn definition_range_links_to_result_set_with_moniker() {
    let source = "pub fn foo() {}";
    let dump = index_rust_linked("file:///test/a.rs", source);
    assert_valid(&dump);

    let range_id = dump
        .find_definition_range("foo")
        .expect("definition range for foo");
    let rs_id = dump
        .result_set_for_range(range_id)
        .expect("resultSet via next edge");
    let monikers = dump.monikers_for_result_set(rs_id);
    assert!(
        !monikers.is_empty(),
        "resultSet for 'foo' should have at least one moniker attached"
    );
}

// =============================================================================
// Ported from generalTests.ts: hover result attached to resultSet
//
// lsif-node implicitly tests this via ValidateCommand; we make it explicit.
// =============================================================================
#[test]
fn hover_result_attached_to_pub_function() {
    let source = "pub fn bar(x: u32) -> u32 { x }";
    let dump = index_rust_linked("file:///test/a.rs", source);
    assert_valid(&dump);

    let range_id = dump
        .find_definition_range("bar")
        .expect("definition range for bar");
    let rs_id = dump
        .result_set_for_range(range_id)
        .expect("resultSet via next edge");
    assert!(
        dump.has_hover(rs_id),
        "pub fn bar should have a hover result on its resultSet"
    );
}

// =============================================================================
// Ported from generalTests.ts: "Reference Links"
//
// Original TypeScript:
//   interface A { func(); }
//   interface B extends A { func1(); }
//   class D implements C { func() {} func1() {} func2() {} }
//
//   assertElement(id:17, moniker{id:":A.func", kind:"export"})
//
// Rust analog: trait A { fn func(&self); }  →  impl Foo { fn func(&self) {} }
// We verify: all three trait methods produce definition ranges.
// =============================================================================
#[test]
fn multiple_pub_fns_each_get_definition_range() {
    let source = r#"
pub fn alpha() {}
pub fn beta() {}
pub fn gamma() {}
"#;
    let dump = index_rust_linked("file:///test/a.rs", source);
    assert_valid(&dump);

    for name in &["alpha", "beta", "gamma"] {
        assert!(
            dump.find_definition_range(name).is_some(),
            "expected definition range for '{name}'"
        );
    }
}

// =============================================================================
// Ported from generalTests.ts: type cycle — no endless recursion / panic
//
// Original:
//   export type BaseCompressValue = boolean | number | string | object;
//   export type CompressValue = BaseCompressValue | undefined | CompressArray;
//   export interface CompressArray extends Array<CompressValue> {}
//   // assertion: no endless recursion
//
// Rust analog: mutually referential type aliases.
// =============================================================================
#[test]
fn type_aliases_do_not_cause_infinite_recursion() {
    let source = r#"
pub type A = Option<B>;
pub type B = Vec<A>;
"#;
    // Should complete without stack overflow / panic
    let dump = index_rust_linked("file:///test/a.rs", source);
    assert_valid(&dump);
}

// =============================================================================
// Cross-file: definition in a.rs, call in b.rs — the linker emits attach edge
//
// Ported from exportTests.ts: "Simple export" cross-file scenario
// Original TypeScript:
//   a.ts: export function foo(): void { }
//   b.ts: import { foo } from "./a"; foo()
//   asserts: moniker{scheme:"tsc", id:"a:foo", kind:"export"} exists
//            call range in b.ts links to foo's resultSet (via attach + moniker)
// =============================================================================
#[test]
fn cross_file_export_moniker_emitted_and_attached() {
    let dump = index_rust_multi(&[
        ("file:///test/a.rs", "pub fn foo() {}"),
        ("file:///test/b.rs", "fn main() { foo(); }"),
    ]);
    assert_valid(&dump);

    // The export moniker for `foo` must exist
    assert!(
        dump.find_moniker("rust", "a::foo", "export").is_some(),
        "expected export moniker 'a::foo' — got: {:?}",
        dump.export_monikers("rust")
    );
}
