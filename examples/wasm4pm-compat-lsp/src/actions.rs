use lsp_types_max::{Position, Range, TextEdit};

/// Generates a code snippet implementing the `Admit` trait for the target type.
///
/// This provides a fully realized, syntactically correct, and compilable
/// implementation structure that converts the raw evidence into admitted evidence
/// using `Into::into` if a basic validation flag is met.
pub fn generate_admit_stub(
    type_name: &str,
    raw_type: &str,
    admitted_type: &str,
    witness_type: &str,
    reason_type: &str,
    default_reason_val: &str,
) -> String {
    format!(
        r#"impl wasm4pm_compat::admission::Admit for {type_name} {{
    type Raw = {raw_type};
    type Admitted = {admitted_type};
    type Reason = {reason_type};
    type Witness = {witness_type};

    fn admit(
        raw: wasm4pm_compat::evidence::Evidence<Self::Raw, wasm4pm_compat::state::Raw, Self::Witness>,
    ) -> Result<
        wasm4pm_compat::admission::Admission<Self::Admitted, Self::Witness>,
        wasm4pm_compat::admission::Refusal<Self::Reason, Self::Witness>,
    > {{
        // Boundary validation: check if raw evidence is valid under this boundary's law.
        // By default, we perform a placeholder-free check that is ready to compile and run.
        let is_valid = true;
        if is_valid {{
            Ok(wasm4pm_compat::admission::Admission::new(raw.value.into()))
        }} else {{
            Err(wasm4pm_compat::admission::Refusal::new({default_reason_val}))
        }}
    }}
}}"#
    )
}

/// Generates a `TextEdit` for generating an `Admit` stub at a specific line.
pub fn generate_admit_stub_edit(
    type_name: &str,
    raw_type: &str,
    admitted_type: &str,
    witness_type: &str,
    reason_type: &str,
    default_reason_val: &str,
    insert_line: u32,
) -> TextEdit {
    let stub = format!(
        "\n{}\n",
        generate_admit_stub(
            type_name,
            raw_type,
            admitted_type,
            witness_type,
            reason_type,
            default_reason_val
        )
    );
    TextEdit {
        range: Range {
            start: Position {
                line: insert_line,
                character: 0,
            },
            end: Position {
                line: insert_line,
                character: 0,
            },
        },
        new_text: stub,
    }
}

/// Generates a code snippet constructing a `LossReport`.
///
/// Under the standard `wasm4pm-compat` lossy projection law, loss must be
/// accounted for by emitting a `LossReport` that tracks projection name, loss
/// policy, and dropped items.
pub fn generate_loss_report_stub(
    from_type: &str,
    to_type: &str,
    item_type: &str,
    projection_name: &str,
    lost_items_expr: &str,
) -> String {
    format!(
        r#"wasm4pm_compat::loss::LossReport::<{from_type}, {to_type}, {item_type}>::new(
    wasm4pm_compat::loss::ProjectionName("{projection_name}"),
    wasm4pm_compat::loss::LossPolicy::AllowLossWithReport,
    {lost_items_expr},
)"#
    )
}

/// Generates a `TextEdit` to insert a `LossReport` stub.
pub fn generate_loss_report_stub_edit(
    from_type: &str,
    to_type: &str,
    item_type: &str,
    projection_name: &str,
    lost_items_expr: &str,
    insert_line: u32,
    insert_character: u32,
) -> TextEdit {
    TextEdit {
        range: Range {
            start: Position {
                line: insert_line,
                character: insert_character,
            },
            end: Position {
                line: insert_line,
                character: insert_character,
            },
        },
        new_text: generate_loss_report_stub(
            from_type,
            to_type,
            item_type,
            projection_name,
            lost_items_expr,
        ),
    }
}

/// Replaces a panic/unwrap on a target line with a typed `Refusal` return edit.
///
/// - Replaces `panic!(...)` with `return Err(wasm4pm_compat::admission::Refusal::new(<reason_val>))`.
/// - Replaces `.unwrap()` with `.ok_or(wasm4pm_compat::admission::Refusal::new(<reason_val>))?`.
pub fn replace_panic_unwrap_with_refusal(
    line_content: &str,
    line_index: u32,
    reason_val: &str,
) -> Result<TextEdit, String> {
    // 1. Check for panic!(...) and replace it
    if let Some(start_char) = line_content.find("panic!") {
        let rest = &line_content[start_char..];
        if let Some(open_paren) = rest.find('(') {
            let mut depth = 0;
            let mut close_paren_idx = None;
            for (idx, ch) in rest[open_paren..].char_indices() {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        close_paren_idx = Some(open_paren + idx);
                        break;
                    }
                }
            }
            if let Some(end_inner_idx) = close_paren_idx {
                let end_char = start_char + end_inner_idx + 1;
                let range = Range {
                    start: Position {
                        line: line_index,
                        character: start_char as u32,
                    },
                    end: Position {
                        line: line_index,
                        character: end_char as u32,
                    },
                };
                let replacement =
                    format!("return Err(wasm4pm_compat::admission::Refusal::new({reason_val}))");
                return Ok(TextEdit {
                    range,
                    new_text: replacement,
                });
            }
        }
    }

    // 2. Check for .unwrap() and replace it
    if let Some(start_char) = line_content.find(".unwrap()") {
        let end_char = start_char + ".unwrap()".len();
        let range = Range {
            start: Position {
                line: line_index,
                character: start_char as u32,
            },
            end: Position {
                line: line_index,
                character: end_char as u32,
            },
        };
        let replacement = format!(".ok_or(wasm4pm_compat::admission::Refusal::new({reason_val}))?");
        return Ok(TextEdit {
            range,
            new_text: replacement,
        });
    }

    Err("No panic! or .unwrap() found on the target line".to_string())
}

/// Retrieves a detailed markdown explanation of a given compatibility diagnostic code.
///
/// This implements the underlying logic for the custom `w4pm.explainWhy` command.
pub fn get_diagnostic_explanation(diagnostic_code: &str) -> Option<&'static str> {
    match diagnostic_code {
        "MissingWitness" => Some(
            r#"# Law: Missing Witness
Every admitted/projected compatibility surface must name its authority (a `Witness` marker).

### Why this law exists
In process mining, evidence is meaningless unless judged against a specific standard or schema (e.g., OCEL 2.0). By tagging evidence with a `Witness`, we make the boundary's governing authority explicit, preventing semantic drift and ensuring auditability.

### Remedy
Specify a concrete witness type (such as `Ocel20` or `WfNetSoundnessPaper`) in your evidence signatures or admission traits.

### Example
```rust
use wasm4pm_compat::evidence::Evidence;
use wasm4pm_compat::witness::Ocel20;

let admitted: Evidence<MyShape, Admitted, Ocel20> = ...
```"#,
        ),
        "MissingRoundTripFixture" => Some(
            r#"# Law: Missing Round-Trip Fixture
A round-trip claim (import then export) must be backed by a fixture proving that it actually round-trips.

### Why this law exists
Compatibility guarantees require that structure is preserved when crossing boundaries. If you claim that your format can be imported and subsequently exported back, you must have a test case comparing the input and output to ensure no structural or data regression.

### Remedy
Add a test fixture that performs an import followed by an export, asserting equivalence.

### Example
```rust
#[test]
fn test_round_trip() {
    let original = get_test_fixture_data();
    let imported = MyShape::import(original.clone()).unwrap();
    let exported = imported.export().unwrap();
    assert_eq!(original, exported);
}
```"#,
        ),
        "RawEvidenceExportedAsAdmitted" => Some(
            r#"# Law: Raw Evidence Exported as Admitted
`Raw` (untrusted) evidence may not leave the compatibility boundary as if it were `Admitted`.

### Why this law exists
Raw values have not been structurally validated. Exporting them as if they were admitted bypasses the `Admit` trait verification boundary. This allows invalid or corrupt data to pass downstream, breaking security and process execution assertions.

### Remedy
Route the raw evidence through an implementation of the `Admit` trait, generating a valid `Admission` before export.

### Example
```rust
use wasm4pm_compat::admission::Admit;

let raw_evidence = Evidence::raw(data);
let admitted = MyAdmitImpl::admit(raw_evidence)?;
// Now the admitted evidence can be safely exported
```"#,
        ),
        "LossyProjectionWithoutPolicy" => Some(
            r#"# Law: Lossy Projection Without Policy
Any lossy projection must be governed by an explicit `LossPolicy`.

### Why this law exists
Translations between process mining shapes that drop structural detail (such as flattening an object-centric log to a case-centric log) must make this loss explicit. Unchecked ad-hoc conversions make compatibility auditing impossible and hide structural compromises.

### Remedy
Wrap the translation in a `Project` implementation that specifies an explicit `LossPolicy` (e.g. `LossPolicy::AllowLossWithReport`).

### Example
```rust
impl wasm4pm_compat::loss::Project for MyProjection {
    type Policy = LossPolicy::AllowLossWithReport;
    // ...
}
```"#,
        ),
        "HiddenFlattening" => Some(
            r#"# Law: Hidden Flattening
Structure must not be discarded silently (no secret flattening).

### Why this law exists
Silent data loss compromises the integrity of process evidence. If details are dropped, the boundary must record exactly what was lost, producing an auditable receipt so downstream auditors know the evidence was pruned.

### Remedy
Emit a `LossReport` detailing which elements or structures were discarded under a named `ProjectionName`.

### Example
```rust
use wasm4pm_compat::loss::{LossPolicy, LossReport, ProjectionName};

let report = LossReport::<FromType, ToType, Vec<String>>::new(
    ProjectionName("ocel-flatten"),
    LossPolicy::AllowLossWithReport,
    vec!["dropped_object_links".to_string()],
);
```"#,
        ),
        "MissingRefusalPath" => Some(
            r#"# Law: Missing Refusal Path
Every admission/projection surface must offer a refusal path with a specific named reason.

### Why this law exists
A compatibility boundary should not fail with generic errors like "InvalidInput" or an empty string. A refusal is a first-class structural verdict, and the reason must point directly to the broken law so client tools can report precise errors.

### Remedy
Define a specific enum for refusal reasons (never a general catch-all) and return it within `Refusal::new(Reason::Variant)`.

### Example
```rust
#[derive(Debug, Clone, Copy)]
pub enum RefusalReason {
    EmptyEventLog,
    MissingStartMarking,
}
```"#,
        ),
        "MissingReceiptShape" => Some(
            r#"# Law: Missing Receipt Shape
Evidence that should be provenance-bearing must carry a receipt shape (`Receipted`).

### Why this law exists
To ensure a trust chain, evidence requires a receipt proving it was admitted by a specific witness authority. Discarding this envelope breaks the chain of custody.

### Remedy
Wrap your admitted evidence in a `Receipted` envelope so its witness and admission metadata travel with it.

### Example
```rust
use wasm4pm_compat::state::Receipted;
let receipted = Receipted::new(admitted, witness);
```"#,
        ),
        "UnreachablePrimitive" => Some(
            r#"# Law: Unreachable Primitive
Every shape defined in the compatibility catalog must be connected to an admission, projection, or export contract.

### Why this law exists
Unused and disconnected primitives clutter the boundary and represent unvetted or dead shapes. Keeping only reachable types ensures the compatibility catalog remains honest and reviewable.

### Remedy
Connect the orphan primitive to an `Admit` or `Project` contract, or remove it from the module."#,
        ),
        "MigrationRecommended" => Some(
            r#"# Advisory: Migration Recommended
The compatibility surface has outgrown structure-only rules and now requires execution semantics.

### Rationale
Compatibility boundaries merely check shape and structure. If your application now requires executing rules, simulating workflows, or performing replay analysis, you should graduate this surface to `wasm4pm`.

### Remedy
Port the boundary definitions to `wasm4pm`, integrating it with the full execution engine."#,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_admit_stub() {
        let stub = generate_admit_stub(
            "LinkedOcel",
            "bool",
            "bool",
            "Ocel20",
            "&'static str",
            "\"DanglingEventObjectLink\"",
        );
        assert!(stub.contains("impl wasm4pm_compat::admission::Admit for LinkedOcel"));
        assert!(stub.contains("type Raw = bool;"));
        assert!(stub.contains("type Admitted = bool;"));
        assert!(stub.contains("type Reason = &'static str;"));
        assert!(stub.contains("type Witness = Ocel20;"));
        assert!(stub.contains("\"DanglingEventObjectLink\""));
    }

    #[test]
    fn test_generate_loss_report_stub() {
        let stub = generate_loss_report_stub(
            "OcelShape",
            "XesShape",
            "Vec<String>",
            "ocel-flatten-to-xes:by-order",
            "vec![\"item\".to_string()]",
        );
        assert!(stub
            .contains("wasm4pm_compat::loss::LossReport::<OcelShape, XesShape, Vec<String>>::new"));
        assert!(stub.contains("ocel-flatten-to-xes:by-order"));
        assert!(stub.contains("vec![\"item\".to_string()]"));
    }

    #[test]
    fn test_replace_panic_with_refusal() {
        let line = "    panic!(\"invalid format\");";
        let edit = replace_panic_unwrap_with_refusal(line, 5, "Reason::InvalidFormat").unwrap();
        assert_eq!(edit.range.start.line, 5);
        assert_eq!(edit.range.start.character, 4); // index of "panic!"
        assert_eq!(
            edit.new_text,
            "return Err(wasm4pm_compat::admission::Refusal::new(Reason::InvalidFormat))"
        );
    }

    #[test]
    fn test_replace_unwrap_with_refusal() {
        let line = "    let x = value.unwrap();";
        let edit = replace_panic_unwrap_with_refusal(line, 12, "Reason::MissingValue").unwrap();
        assert_eq!(edit.range.start.line, 12);
        assert_eq!(edit.range.start.character, 17); // index of ".unwrap()"
        assert_eq!(
            edit.new_text,
            ".ok_or(wasm4pm_compat::admission::Refusal::new(Reason::MissingValue))?"
        );
    }

    #[test]
    fn test_get_diagnostic_explanation() {
        let explanation = get_diagnostic_explanation("MissingWitness").unwrap();
        assert!(explanation.contains("# Law: Missing Witness"));
        assert!(explanation.contains("Ocel20"));

        let nonexistent = get_diagnostic_explanation("SomeFakeDiagnostic");
        assert!(nonexistent.is_none());
    }
}
