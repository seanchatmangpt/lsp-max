# MAX-001 Specgen Metamodel Agent Report

## Status
MAX_IMPLEMENTATION_COMPLETE

## MetaModel Fixtures
The following metamodel fixtures are present in the `crates/tower-lsp-max-specgen/fixtures/` directory:
1.  **Official LSP 3.18 MetaModel Fixture:**
    *   **File Name:** `metaModel-3.18.json`
    *   **Path:** `crates/tower-lsp-max-specgen/fixtures/metaModel-3.18.json`
    *   **Size:** 434,246 bytes
    *   **Details:** Confirmed via internal metadata to represent LSP version `3.18.0`. It contains the complete schema specification including `requests`, `notifications`, `structures`, `enumerations`, and `typeAliases`.
2.  **Minimal MetaModel Fixture:**
    *   **File Name:** `minimal-metaModel.json`
    *   **Path:** `crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json`
    *   **Size:** 889 bytes
    *   **Details:** Used for fast-path boot testing and validation of the generator toolchain.

Both fixtures are successfully read and parsed using `serde_json` into the `MetaModel` AST.

## Supported Type Kinds
The parser explicitly supports all 11 type kinds defined by the official Language Server Protocol meta-model. These are declared inside the `Type` enum in `crates/tower-lsp-max-specgen/src/metamodel.rs`:
*   `Base { name: BaseTypeName }`: Maps core types (e.g., `URI`, `DocumentUri`, `integer`, `string`, `boolean`).
*   `Reference { name: String }`: References to other named structures, enums, or type aliases.
*   `Array { element: Box<Type> }`: Arrays of other types.
*   `Map { key: MapKeyType, value: Box<Type> }`: Map/dictionary types.
*   `And { items: Vec<Type> }`: Intersection/intersection-like type combinations.
*   `Or { items: Vec<Type> }`: Type unions (sum types).
*   `Tuple { items: Vec<Type> }`: Fixed-size arrays or tuple forms.
*   `StringLiteral { value: String }`: Types containing a specific string literal value constraint.
*   `IntegerLiteral { value: i64 }`: Types containing a specific integer literal value constraint.
*   `BooleanLiteral { value: bool }`: Types containing a specific boolean literal value constraint.
*   `Literal { value: StructureLiteral }`: Inline/anonymous structure definition.

## Unsupported / Conservative Lowerings
Although the memory-resident AST (the "known law") tracks all complex type structures, the code generation/rendering layer (`src/render.rs`) applies conservative lowerings that collapse structural details:
1.  **Union & Intersection Collapse (`Or`, `And`):**
    *   Lowered to the generic `LspAny` (`serde_json::Value`) instead of generating named/anonymous Rust enums or combining properties.
    *   *Example:* `pub type Definition = LspAny;`
2.  **Tuple Collapse (`Tuple`):**
    *   Lowered directly to `LspAny` instead of Rust tuples (e.g. `(TypeA, TypeB)`).
3.  **Inline Structure Collapse (`Literal`):**
    *   Anonymous structures defined inline within properties are collapsed to `LspAny` rather than creating distinct nested structs.
4.  **Literal Value Generalization:**
    *   `StringLiteral`, `IntegerLiteral`, and `BooleanLiteral` are mapped to generic primitive types (`String`, `Integer`, `bool`), erasing the specific value boundary constraints.
5.  **Map Key Simplification:**
    *   The keys for maps are unconditionally serialized to `String`, ignoring the specific `MapKeyType` defined in the metamodel (e.g. integer or reference keys).

## Generator Commands
The specification generator is invoked with the following commands to produce Rust code:
1.  **Minimal LSP Specification Generation:**
    ```bash
    cargo run -p tower-lsp-max-specgen -- --input crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json --output generated/lsp_minimal.rs
    ```
2.  **Full LSP 3.18.0 Specification Generation:**
    ```bash
    cargo run -p tower-lsp-max-specgen -- --input crates/tower-lsp-max-specgen/fixtures/metaModel-3.18.json --output generated/lsp_3_18.rs
    ```

## Required Follow-up Gates
To transition from conservative lowerings to complete type safety, the following gates must be resolved in future iterations:
1.  **AST-to-Rust Enum Generator for Unions (`Or`):** Design a mapping layer that translates unions (e.g., `Location | Location[]`) into well-defined Rust enums with appropriate serialization/deserialization implementations.
2.  **Anonymous Structure Extractor:** Parse inline `Literal` structures and generate named Rust structs, referencing them dynamically in properties.
3.  **Exact-Value Constraints:** Implement custom validation or custom types for string/integer literals (e.g., specific string kind enums) to enforce protocol-level contract boundaries.
