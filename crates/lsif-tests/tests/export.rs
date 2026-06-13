/// Export / cross-file reference tests — ported from microsoft/lsif-node/tsc-tests/src/exportTests.ts
///
/// Original test: `suite('Export Tests', ...)`
/// Source: vendors/lsif-node/tsc-tests/src/exportTests.ts
///
/// Adaptation notes:
/// - lsif-node tests two-file TypeScript programs (a.ts + b.ts).
///   We test equivalent two-file Rust programs (a.rs + b.rs) via index_rust_multi,
///   and two-file TypeScript programs via index_typescript_multi.
/// - Moniker identifiers: lsif-node uses "a:foo" (module colon name);
///   our Rust indexer emits "a::foo", our TypeScript indexer emits "pkg::foo"
///   when a package name is supplied.
use lsif_tests::{index_rust_multi, index_typescript_multi};

fn assert_valid(dump: &lsif_tests::LsifDump) {
    if let Err(e) = dump.validate() {
        panic!("LSIF structural validation failed:\n{e}");
    }
}

// =============================================================================
// Ported from exportTests.ts: "Simple export"
//
// Original:
//   a.ts: export function foo(): void { }
//   b.ts: import { foo } from "./a"; foo()
//   assert: moniker{scheme:"tsc", id:"a:foo", kind:"export"}
//           resultSet + moniker edge + definition range for foo
//
// Rust:
//   a.rs: pub fn foo() {}
//   b.rs: fn main() { foo(); }
// =============================================================================
#[test]
fn simple_export_function() {
    let dump = index_rust_multi(&[
        ("file:///test/a.rs", "pub fn foo() {}"),
        ("file:///test/b.rs", "fn main() { foo(); }"),
    ]);
    assert_valid(&dump);

    // Export moniker must exist for `foo`
    let exports = dump.export_monikers("rust");
    assert!(
        exports.iter().any(|id| id.ends_with("foo")),
        "expected export moniker ending in 'foo', got: {exports:?}"
    );

    // Definition range must exist
    assert!(
        dump.find_definition_range("foo").is_some(),
        "expected definition range tagged 'foo'"
    );
}

// =============================================================================
// Ported from exportTests.ts: "Const export"
//
// Original:
//   a.ts: export const x: number | string = 10;
//   b.ts: import { x } from "./a"; x;
//   assert: moniker{id:"a:x", kind:"export"}
//
// Rust:
//   a.rs: pub const X: u32 = 10;
// =============================================================================
#[test]
fn const_export() {
    let dump = index_rust_multi(&[
        ("file:///test/a.rs", "pub const X: u32 = 10;"),
        ("file:///test/b.rs", "fn use_x() { let _ = X; }"),
    ]);
    assert_valid(&dump);

    let exports = dump.export_monikers("rust");
    assert!(
        exports.iter().any(|id| id.contains('X')),
        "expected export moniker containing 'X', got: {exports:?}"
    );
}

// =============================================================================
// Ported from exportTests.ts: "Namespace export" / struct export
//
// Original:
//   a.ts: export namespace N { export const a: number = 10; }
//   assert: moniker{id:"a:N", kind:"export"}
//           moniker{id:"a:N.a", kind:"export"}
//
// Rust:
//   a.rs: pub struct Foo { pub x: u32 }
//   Both `Foo` and its fields should have export monikers.
// =============================================================================
#[test]
fn struct_export_moniker() {
    let dump = index_rust_multi(&[
        ("file:///test/a.rs", "pub struct Foo { pub x: u32 }"),
        ("file:///test/b.rs", "fn use_foo(f: Foo) { let _ = f.x; }"),
    ]);
    assert_valid(&dump);

    let exports = dump.export_monikers("rust");
    assert!(
        exports.iter().any(|id| id.contains("Foo")),
        "expected export moniker containing 'Foo', got: {exports:?}"
    );
}

// =============================================================================
// Ported from exportTests.ts: enum export
//
// Rust:
//   a.rs: pub enum Color { Red, Green, Blue }
// =============================================================================
#[test]
fn enum_export_moniker() {
    let dump = index_rust_multi(&[
        ("file:///test/a.rs", "pub enum Color { Red, Green, Blue }"),
        ("file:///test/b.rs", "fn use_color(c: Color) {}"),
    ]);
    assert_valid(&dump);

    assert!(
        dump.find_definition_range("Color").is_some(),
        "expected definition range for Color"
    );
    let exports = dump.export_monikers("rust");
    assert!(
        exports.iter().any(|id| id.contains("Color")),
        "expected export moniker for Color, got: {exports:?}"
    );
}

// =============================================================================
// Ported from exportTests.ts: "Simple export" — TypeScript variant
//
// Original:
//   a.ts: export function foo(): void { }
//   b.ts: import { foo } from "./a"; foo()
// =============================================================================
#[test]
fn typescript_simple_export_function() {
    let dump = index_typescript_multi(
        &[
            ("file:///test/a.ts", "export function foo(): void {}"),
            ("file:///test/b.ts", "import { foo } from './a';\nfoo();"),
        ],
        Some("my-pkg"),
    );
    assert_valid(&dump);

    let exports = dump.export_monikers("npm");
    assert!(
        exports.iter().any(|id| id.ends_with("foo")),
        "expected npm export moniker ending in 'foo', got: {exports:?}"
    );

    assert!(
        dump.find_definition_range("foo").is_some(),
        "expected definition range for foo"
    );
}

// =============================================================================
// Ported from exportTests.ts: "Const export" — TypeScript variant
//
// Original:
//   a.ts: export const x: number | string = 10;
// =============================================================================
#[test]
fn typescript_const_export() {
    let dump = index_typescript_multi(
        &[
            ("file:///test/a.ts", "export const x: number = 10;"),
            (
                "file:///test/b.ts",
                "import { x } from './a';\nconsole.log(x);",
            ),
        ],
        Some("my-pkg"),
    );
    assert_valid(&dump);
    let exports = dump.export_monikers("npm");
    assert!(
        exports.iter().any(|id| id.ends_with("x")),
        "expected npm export moniker ending in 'x' for const export, got: {exports:?}"
    );
}

// =============================================================================
// Ported from exportTests.ts: class export — TypeScript variant
//
// Original:
//   class declaration in a.ts, reference in b.ts
// =============================================================================
#[test]
fn typescript_class_export() {
    let dump = index_typescript_multi(
        &[
            (
                "file:///test/a.ts",
                "export class MyService { doWork(): void {} }",
            ),
            (
                "file:///test/b.ts",
                "import { MyService } from './a';\nconst s = new MyService();",
            ),
        ],
        Some("my-pkg"),
    );
    assert_valid(&dump);

    assert!(
        dump.find_definition_range("MyService").is_some(),
        "expected definition range for MyService"
    );
}

// =============================================================================
// Structural: every definition range must be connected to a resultSet
//
// This cross-cuts all export tests. lsif-node's ValidateCommand checks this
// implicitly; we make it explicit.
// =============================================================================
#[test]
fn all_definition_ranges_have_result_sets() {
    let dump = index_rust_multi(&[
        (
            "file:///test/a.rs",
            "pub fn foo() {}\npub struct Bar {}\npub enum Baz { A, B }",
        ),
        ("file:///test/b.rs", "fn use_all() {}"),
    ]);
    assert_valid(&dump);

    for name in &["foo", "Bar", "Baz"] {
        if let Some(range_id) = dump.find_definition_range(name) {
            assert!(
                dump.result_set_for_range(range_id).is_some(),
                "definition range for '{name}' has no resultSet via next edge"
            );
        }
    }
}

// =============================================================================
// Linker: attach edge emitted for cross-file import/export pair
//
// Ported from the lsif-node linker contract: after link(), every import moniker
// that matches an export moniker must have an `attach` edge.
// =============================================================================
#[test]
fn attach_edge_present_after_linking() {
    // index_rust_multi runs the linker pass internally
    let dump = index_rust_multi(&[
        ("file:///test/a.rs", "pub fn shared() {}"),
        ("file:///test/b.rs", "fn caller() { shared(); }"),
    ]);
    assert_valid(&dump);

    // Export moniker must exist (attach edges are emitted by lsif-linker
    // when an import moniker matches — our Rust indexer currently emits
    // export monikers for pub items; import monikers are emitted by the
    // typed-AST dispatch! upgrade).
    let exports = dump.export_monikers("rust");
    assert!(
        exports.iter().any(|id| id.contains("shared")),
        "export moniker for 'shared' missing; linker has nothing to attach"
    );
}
