# MAX-002 Lowering Conformance Report

## Status
`MAX_POLICY_FORMULATED`

## Environment Details
- **Timestamp:** 2026-06-04T21:28:20-07:00
- **Operating System:** mac (Apple macOS)
- **Rust Toolchain:** stable-x86_64-apple-darwin
- **Target LSP Specification Version:** LSP 3.18.0

---

## 1. LSP 3.18.0 Meta-Model Inventory

An inspection of `crates/lsp-max-specgen/fixtures/metaModel-3.18.json` reveals the following distribution of protocol definitions:
*   **Structures:** 387
*   **Enumerations:** 40
*   **Type Aliases:** 22
*   **Requests (RPC Methods):** 69
*   **Notifications (RPC Methods):** 26

These definitions contain complex types, including:
*   **Unions (`Or`):** 136 occurrences (e.g. `ProgressToken = integer | string`)
*   **Intersections (`And`):** 1 occurrence (registration options for `textDocument/willSaveWaitUntil`)
*   **Tuples (`Tuple`):** 1 occurrence (`ParameterInformation.label` offsets)
*   **Anonymous Inline Structures (`Literal`):** 2 occurrences (empty literals `{}` representing options in semantic tokens)
*   **Maps (`Map`):** 6 occurrences (e.g. changes maps in workspace edits)

---

## 2. Generator Architecture Analysis

The `lsp-max-specgen` generator parses the meta-model JSON and outputs formatted Rust structures. It consists of three main source modules:
*   `src/metamodel.rs`: Represents the AST schema of the official LSP meta-model. Defines the 11 type kinds inside the `Type` enum: `Base`, `Reference`, `Array`, `Map`, `And`, `Or`, `Tuple`, `StringLiteral`, `IntegerLiteral`, `BooleanLiteral`, and `Literal`.
*   `src/render.rs`: Maps the parsed AST into Rust code using the `quote` macro.
*   `src/main.rs`: Provides the CLI wrapper interface to read the input JSON, invoke `Renderer`, and write the output using `prettyplease`.

---

## 3. Silent & Undocumented Fallbacks

During the investigation, several silent or undocumented fallbacks to `LspAny` (`serde_json::Value`) or primitive types were discovered in `src/render.rs`:

1.  **Complex Type Collapse (`Type::And`, `Type::Or`, `Type::Tuple`, `Type::Literal`):**
    *   **Location:** `src/render.rs:396-398`
    *   **Fallback:** Collapses intersections, unions, tuples, and anonymous literals directly to `LspAny`.
    *   **Identified Instances:**
        *   **Intersection (`And`):** Request `textDocument/willSaveWaitUntil` registration options type is defined as `And(WorkDoneProgressOptions, TextDocumentRegistrationOptions)`. Lowered to `LspAny`.
        *   **Tuple (`Tuple`):** `ParameterInformation.label` contains a union item `Tuple(uinteger, uinteger)`. Since it is part of a union type, the entire property is lowered to `LspAny`.
        *   **Literal (`Literal`):** `SemanticTokensOptions.range` and `ClientSemanticTokensRequestOptions.range` use an empty literal `Literal()` inside a union (`boolean | Literal()`). The entire property is lowered to `LspAny`.
    *   **Impact:** 14 out of 22 type aliases and 64 structure properties are silently lowered to `LspAny`, eliminating compile-time safety for these fields.

2.  **Multiple Request/Notification Parameter Fallback (`OneOrManyTypes::Many`):**
    *   **Location:** `src/render.rs:309` and `src/render.rs:350`
    *   **Fallback:** If request or notification parameters are defined as a list of multiple types (`Many`), they are lowered to `LspAny`.
    *   **Impact:** Any multi-parameter request/notification loses typed arguments. (Note: No instances of `Many` parameters currently exist in the LSP 3.18.0 metamodel; all methods declare at most a single parameter type or none).

3.  **Map Key Type Coercion:**
    *   **Location:** `src/render.rs:391-394`
    *   **Fallback:** Unconditionally sets the key of all maps to `String` (e.g. `std::collections::BTreeMap<String, Value>`), ignoring the specific `MapKeyType` parsed from the AST.
    *   **Identified Instances:**
        *   `WorkspaceEdit.changes`: defined as `Map` with key `DocumentUri` and value `Array(TextEdit)`. Lowered to `BTreeMap<String, Vec<TextEdit>>`.
        *   `WorkspaceEdit.changeAnnotations`: defined as `Map` with key `ChangeAnnotationIdentifier` and value `ChangeAnnotation`. Lowered to `BTreeMap<String, ChangeAnnotation>`.
        *   `DocumentDiagnosticReportPartialResult.relatedDocuments`, `RelatedFullDocumentDiagnosticReport.relatedDocuments`, and `RelatedUnchangedDocumentDiagnosticReport.relatedDocuments`: defined as `Map` with key `DocumentUri` and value `FullDocumentDiagnosticReport | UnchangedDocumentDiagnosticReport`. Lowered to `BTreeMap<String, LspAny>`.
        *   `LSPObject`: defined as `Map` with key `string` and value `LSPAny`. Lowered to `BTreeMap<String, LspAny>`.
    *   **Impact:** Maps keyed by `DocumentUri` or `ChangeAnnotationIdentifier` lose type specificity, falling back to raw `String`.

4.  **Literal Value Generalization:**
    *   **Location:** `src/render.rs:399-401`
    *   **Fallback:** `StringLiteral`, `IntegerLiteral`, and `BooleanLiteral` kinds are generalized to `String`, `Integer` (i32), and `bool`, discarding compile-time literal constraints.
    *   **Impact:** Constraints (e.g. fields restricted to a specific string tag) are not enforced at the type level.

5.  **Dead Integer Enum Path:**
    *   **Location:** `src/render.rs:153-154`, `161`, and `184-280`
    *   **Fallback:** The `open` flag is evaluated as:
        ```rust
        let open = en.supports_custom_values.unwrap_or(false) || en.ty.name != EnumerationBaseType::String;
        ```
        Any integer or uinteger enumeration will have `en.ty.name != EnumerationBaseType::String`, making `open == true`. Consequently, all integer enums (including closed ones like `SymbolKind`, `InlayHintKind`, etc.) are generated as transparent newtype structures, leaving the custom serialization/deserialization logic for integer enums in the `else` branch (lines 184-280) completely unreachable (dead code).

6.  **Silently Ignored Extends and Mixins:**
    *   **Location:** `src/render.rs:79-98`
    *   **Fallback:** The parser iterates over `extends` and `mixins`, but only acts `if let Type::Reference { name } = ext`.
    *   **Impact:** If a future LSP version uses complex types (like unions) inside `extends` or `mixins`, they will be silently omitted without compiler warnings or runtime errors.

---

## 4. Explicit Lowering Policies

To establish robust conformance and enable transition to full type safety, the following lowering policies are explicitly defined:

### A. Native Rust Type Policy
Primitive types from the LSP meta-model must map directly to standard Rust types and primitives:
*   `URI` $\rightarrow$ `pub type URI = String;`
*   `DocumentUri` $\rightarrow$ `pub type DocumentUri = String;`
*   `integer` $\rightarrow$ `pub type Integer = i32;`
*   `uinteger` $\rightarrow$ `pub type Uinteger = u32;`
*   `decimal` $\rightarrow$ `pub type Decimal = f64;`
*   `RegExp` $\rightarrow$ `pub type RegExp = String;`
*   `string` $\rightarrow$ `String`
*   `boolean` $\rightarrow$ `bool`
*   `null` $\rightarrow$ `()`

### B. Boxed Recursive Type Policy
Recursive layouts must be broken using heap allocation to prevent infinite stack size:
*   **Direct self-reference:** If a struct property references its own containing struct (e.g. `SelectionRange` having a `parent` of type `SelectionRange`), the field type must be wrapped in a `Box<T>`.
*   **Indirect cycle:** If a cycle exists through multiple structures (e.g., $A \rightarrow B \rightarrow A$ with no intervening heap allocation), the cycle must be identified and at least one edge broken by wrapping the reference in `Box<T>`.
    *   *Note:* The LSP 3.18.0 metamodel contains exactly two recursive structural relationships:
        1.  `SelectionRange -> SelectionRange` (direct recursion on property `parent`, must be boxed as `Option<Box<SelectionRange>>`).
        2.  `DocumentSymbol -> DocumentSymbol` (recursion through array `children`, represented as `Option<Vec<DocumentSymbol>>` which is heap-allocated and does not require stack-level boxing).
        No multi-structure indirect cycles exist in the LSP 3.18.0 metamodel.
    *   *Note:* References wrapped in `Vec<T>`, `std::collections::BTreeMap<K, V>`, or types lowered to `LspAny` are already heap-allocated and do not require boxing.

### C. Transparent Newtype Policy
Used for open enumerations that permit custom values or rely on integer/uinteger types:
*   Renders as:
     ```rust
     #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
     #[serde(transparent)]
     pub struct EnumName(pub BaseType);
     ```
*   Known values must be rendered as associated constants on the struct:
     ```rust
     impl EnumName {
         pub const KNOWN_VAL_A: BaseType = ...;
     }
     ```

### D. Tagged / Untagged Enum Policy
Used for closed enumerations and resolved union types (`Or`):
*   **Closed String Enums:** Rendered as standard Rust enums with `#[derive(Serialize, Deserialize)]` and rename attributes mapping Rust camel-case to LSP string values.
*   **Closed Integer Enums:** Renders as standard Rust enums with custom `Serialize` and `Deserialize` implementations mapping enum variants to exact integer values (reclaiming the dead code path).
*   **Simple Type Unions (`Or`):**
    *   If the union represents `T | null`, lower to `Option<T>`.
    *   If the union represents multiple distinct named types (e.g., `Location | Location[]`), lower to an untagged enum:
        ```rust
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(untagged)]
        pub enum LocationOrLocations {
            Single(Location),
            Multiple(Vec<Location>),
        }
        ```

### E. Intentional LspAny / serde_json::Value Fallback Policy
Polymorphic fields and dynamic payloads that are truly unbounded by the LSP specification are intentionally lowered to `serde_json::Value`:
*   The type alias `LSPAny` in the meta-model maps to `pub type LSPAny = LspAny;`.
*   Any payload defined as generic or arbitrary JSON in the spec (such as client/server experimental capabilities or request/notification `data` fields) is lowered to `LspAny`.

### F. Refused / Unsupported Form Policy
To prevent silent code generation failures or unsafe representations, the generator must reject or error on:
*   **Intersections (`And`):** Intersections where properties overlap incompatibly, or intersections containing non-structural types (e.g. `string & number`), must be refused.
*   **Unconvertible Map Keys:** Any map key that is not a string, primitive integer, or alias thereto must be rejected.
*   **Non-Reference Extends/Mixins:** Complex types (e.g. unions or arrays) defined within `extends` or `mixins` must be rejected with an explicit compilation error.

---

## 5. Next Steps / Actionable Roadmap

1.  **Refactor `render_enumeration`:** Remove the dead code and cleanly partition open enums (which use transparent newtypes) from closed enums (which use custom serialized/deserialized Rust enums).
2.  **Introduce Union-to-Enum Lowering:** Replace `Or` collapse logic with a pass that generates named helper untagged enums.
3.  **Introduce Map Key Validation:** Validate map keys and generate compiler errors if unsupported key types are encountered.
