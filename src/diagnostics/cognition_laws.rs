//! Law table extension: periodic table of cognition breed correctness.
//!
//! Emits diagnostics (A8–A12 adversary classes + structural completeness) for every
//! PARTIAL_ALIVE breed in ../wasm4pm/crates/wasm4pm-cognition/breeds/registry.json.
//! Designed to run as a workspace-level diagnostic gate — integrates with `update_diagnostics`
//! and the lsp-max law engine.
//!
//! Laws enforced:
//!   COG-001  Missing breed module (.rs file)
//!   COG-002  Missing OCPN model (ocel/models/l1/`<breed>`.ocpn.json)
//!   COG-003  Missing OCEL fitness report (ocel/reports/`<breed>`.json)
//!   COG-004  Missing paper fixture (tests/fixtures/papers/`<breed>`.json)
//!   COG-005  Fixture missing expected.value — citation without assertion (A12)
//!   COG-006  OCEL report fitness != 1.0
//!   COG-007  OCEL report missing measured-fitness provenance (A10 evidence gap)
//!   COG-008  Missing docs card (docs/breeds/`<breed>`.md)
//!   COG-009  Missing TS fixture mirror (packages/cognition/src/__tests__/fixtures/papers/)
//!   COG-010  Oracle identifier leaked into production source (A8 — fresh-name violation)
//!   COG-011  Premature PARTIAL_ALIVE: registry flipped but required artifacts absent (A10)
//!   COG-012  Missing BreedId dispatch arm (breed not wired in dispatch.rs)

use lsp_types_max::DiagnosticSeverity;
use std::path::Path;

/// Severity mapping per law — matches the ARD's defect classification.
fn severity(law: &str) -> DiagnosticSeverity {
    match law {
        // Hard failures — breed cannot be certified without these
        "COG-001" | "COG-002" | "COG-004" | "COG-010" | "COG-011" | "COG-012" => {
            DiagnosticSeverity::ERROR
        }
        // Evidence gaps — certification claim weakened
        "COG-003" | "COG-005" | "COG-006" | "COG-007" => DiagnosticSeverity::ERROR,
        // Completeness warnings — ceremony not fully done
        "COG-008" | "COG-009" => DiagnosticSeverity::WARNING,
        _ => DiagnosticSeverity::INFORMATION,
    }
}

/// One diagnostic finding for a single breed.
#[derive(Debug, Clone)]
pub struct BreedDiagnostic {
    /// Law identifier, e.g. `"COG-001"`.
    pub law_id: &'static str,
    /// The breed id that violated the law.
    pub breed_id: String,
    /// Human-readable diagnostic message.
    pub message: String,
    /// LSP severity level.
    pub severity: DiagnosticSeverity,
    /// Path of the missing/violating artifact (for LSP range attachment).
    pub artifact_path: String,
}

/// Derive the wasm4pm workspace root from a given `root_path`.
/// Handles both "we're inside wasm4pm" and "we're in lsp-max with ../wasm4pm sibling".
fn wasm4pm_root(root_path: &Path) -> std::path::PathBuf {
    // If registry.json exists here, we're already inside wasm4pm.
    let direct = root_path.join("crates/wasm4pm-cognition/breeds/registry.json");
    if direct.exists() {
        return root_path.to_path_buf();
    }
    // Otherwise assume sibling checkout.
    root_path.parent().unwrap_or(root_path).join("wasm4pm")
}

/// Known hidden-oracle fresh names per breed (from anti-cheat-threat-model.md).
/// If any of these appear in the production breed source, it is an A8 violation.
fn fresh_name_manifest(breed_id: &str) -> Vec<&'static str> {
    match breed_id {
        "ltl_monitor" => vec!["zorp", "blee", "quux"],
        "allen_temporal" => vec!["gamma", "delta", "eps", "pi"],
        "fuzzy_logic" => vec!["tri_asymmetric", "flam_var"],
        "bayesian_network" => vec!["qubit", "qres", "qchain"],
        "csp_ac3" => vec!["vquux", "vblee", "vzorp"],
        "default_logic" => vec!["gronk", "wibble", "dark_wibble"],
        "htn_planning" => vec!["coach_task", "walk_task"],
        "dempster_shafer" => vec!["flim", "flam"],
        "frames_inheritance" => vec!["zilk", "welp", "snorf"],
        "ebl" => vec!["obj2", "obj9"],
        "asp" => vec!["zorp_atom", "blee_atom"],
        "description_logic" => vec!["krumm", "blurp"],
        "abductive_lp" => vec!["snag", "blarg"],
        "circumscription" => vec!["korv", "glows"],
        "analogy_sme" => vec!["gor", "lum", "rix"],
        "naive_physics" => vec!["bolv", "mim", "pearl"],
        "problog" => vec!["pfact_quux", "pfact_blee"],
        "pomdp" => vec!["tampered_o"],
        "meta_reasoning" => vec!["breed_zorp", "breed_blee"],
        _ => vec![],
    }
}

/// Parse dispatch.rs to build a map of breed_id → module stem that actually
/// implements it.  Falls back to `breed_id` itself when no override is found.
/// This is the source-of-truth derivation: convention-free, self-updating.
///
/// Two-pass approach:
/// 1. Parse all `module::Struct` tokens from use declarations to build struct→module.
/// 2. Parse `"breed_id" => run_breed(&Struct, ...)` match arms to bind breed_id→module.
fn dispatch_module_map(dispatch_src: &str) -> std::collections::BTreeMap<String, String> {
    let mut struct_to_module: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();

    // Pass 1: extract all `module::Struct` tokens from the source.
    // Split the entire file on whitespace/commas/braces/parens to get tokens.
    for token in dispatch_src.split(|c: char| {
        c == ','
            || c == '{'
            || c == '}'
            || c == '('
            || c == ')'
            || c == '\n'
            || c == '\t'
            || c == ' '
    }) {
        let token = token.trim();
        // Skip comment fragments
        if token.starts_with("//") {
            continue;
        }
        if let Some(pos) = token.find("::") {
            let module_part = token[..pos].trim();
            let struct_part = token[pos + 2..].trim();
            // module must be all lowercase snake_case; struct must start with uppercase
            if !module_part.is_empty()
                && !struct_part.is_empty()
                && module_part.chars().all(|c| c.is_alphanumeric() || c == '_')
                && struct_part.chars().next().is_some_and(|c| c.is_uppercase())
                && module_part != "crate"
                && module_part != "super"
                && module_part != "self"
            {
                struct_to_module
                    .entry(struct_part.to_string())
                    .or_insert_with(|| module_part.to_string());
            }
        }
    }

    // Pass 2: match `"breed_id" => run_breed(&StructName, ...)` or `"breed_id" => StructName.run(...)` arms.
    let mut map = std::collections::BTreeMap::new();
    for line in dispatch_src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                let breed_id = &rest[..end];
                let after = rest[end + 1..].trim();
                if let Some(arm_raw) = after.strip_prefix("=>") {
                    let arm = arm_raw.trim();
                    // `run_breed(&StructName, ...)` or `StructName.run(...)`
                    let struct_name = if let Some(inner) = arm.strip_prefix("run_breed(&") {
                        inner.split(',').next().unwrap_or("").trim().to_string()
                    } else {
                        arm.split(['.', '(', ' '])
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    };
                    if !struct_name.is_empty() {
                        let module =
                            struct_to_module
                                .get(&struct_name)
                                .cloned()
                                .unwrap_or_else(|| {
                                    // Convention fallback: PascalCase → snake_case
                                    {
                                        let mut snake =
                                            String::with_capacity(struct_name.len() + 4);
                                        for (i, c) in struct_name.chars().enumerate() {
                                            if c.is_uppercase() && i > 0 {
                                                snake.push('_');
                                                snake.push(c.to_ascii_lowercase());
                                            } else {
                                                snake.push(c.to_ascii_lowercase());
                                            }
                                        }
                                        snake
                                    }
                                });
                        map.insert(breed_id.to_string(), module);
                    }
                }
            }
        }
    }
    map
}

/// Run all cognition breed laws against the workspace at `root_path`.
/// Returns a list of diagnostics — empty means all laws satisfied.
pub fn audit_breeds(root_path: &Path) -> Vec<BreedDiagnostic> {
    let wasm4pm = wasm4pm_root(root_path);
    let registry_path = wasm4pm.join("crates/wasm4pm-cognition/breeds/registry.json");

    let registry_json = match std::fs::read_to_string(&registry_path) {
        Ok(s) => s,
        Err(_) => return vec![], // registry absent — not a wasm4pm workspace, skip silently
    };

    let registry: serde_json::Value = match serde_json::from_str(&registry_json) {
        Ok(v) => v,
        Err(_) => {
            return vec![BreedDiagnostic {
                law_id: "COG-000",
                breed_id: "registry".into(),
                message: "breeds/registry.json is not valid JSON".into(),
                severity: DiagnosticSeverity::ERROR,
                artifact_path: registry_path.to_string_lossy().into(),
            }]
        }
    };

    // Derive module locations from dispatch.rs — source-of-truth, not convention.
    let dispatch_path = wasm4pm.join("crates/wasm4pm-cognition/src/breeds/dispatch.rs");
    let dispatch_src = std::fs::read_to_string(&dispatch_path).unwrap_or_default();
    let module_map = dispatch_module_map(&dispatch_src);

    // Support both array-of-objects and {breeds: [...]} shapes.
    let breeds_val = registry.get("breeds").unwrap_or(&registry);

    let breeds = match breeds_val.as_array() {
        Some(a) => a.clone(),
        None => {
            if let Some(obj) = breeds_val.as_object() {
                obj.iter()
                    .map(|(k, v)| {
                        let mut entry = v.clone();
                        if let Some(o) = entry.as_object_mut() {
                            o.entry("breed_id")
                                .or_insert(serde_json::Value::String(k.clone()));
                        }
                        entry
                    })
                    .collect()
            } else {
                return vec![];
            }
        }
    };

    let mut diags: Vec<BreedDiagnostic> = Vec::new();

    for breed in &breeds {
        let bid = match breed
            .get("breed_id")
            .and_then(|v| v.as_str())
            .or_else(|| breed.get("id").and_then(|v| v.as_str()))
        {
            Some(s) => s.to_string(),
            None => continue,
        };

        let status = breed
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");

        // Only audit breeds that claim to be implemented.
        if status != "PARTIAL_ALIVE" {
            continue;
        }

        // Derive module path from dispatch.rs parse — not from naming convention.
        let module_stem = module_map.get(&bid).cloned().unwrap_or_else(|| bid.clone());
        let module_path_str = format!("crates/wasm4pm-cognition/src/breeds/{module_stem}.rs");
        let module = wasm4pm.join(&module_path_str);
        let ocpn = wasm4pm.join(format!("ocel/models/l1/{bid}.ocpn.json"));
        let report = wasm4pm.join(format!("ocel/reports/{bid}.json"));
        let fix_rs = wasm4pm.join(format!(
            "crates/wasm4pm-cognition/tests/fixtures/papers/{bid}.json"
        ));
        let fix_ts = wasm4pm.join(format!(
            "packages/cognition/src/__tests__/fixtures/papers/{bid}.json"
        ));
        let doc = wasm4pm.join(format!("docs/breeds/{bid}.md"));

        // COG-001: breed module missing
        if !module.exists() {
            diags.push(BreedDiagnostic {
                law_id: "COG-001",
                breed_id: bid.clone(),
                message: format!("[COG-001] PARTIAL_ALIVE breed '{bid}' has no implementation module — {module_path_str} missing"),
                severity: severity("COG-001"),
                artifact_path: module.to_string_lossy().into(),
            });
        }

        // COG-002: OCPN model missing
        if !ocpn.exists() {
            diags.push(BreedDiagnostic {
                law_id: "COG-002",
                breed_id: bid.clone(),
                message: format!("[COG-002] '{bid}' OCPN model missing — ocel/models/l1/{bid}.ocpn.json required for conformance gate"),
                severity: severity("COG-002"),
                artifact_path: ocpn.to_string_lossy().into(),
            });
        }

        // COG-003 / COG-006 / COG-007: report missing or invalid
        if !report.exists() {
            diags.push(BreedDiagnostic {
                law_id: "COG-003",
                breed_id: bid.clone(),
                message: format!("[COG-003] '{bid}' OCEL fitness report missing — ocel/reports/{bid}.json must be earned (A10 evidence gap)"),
                severity: severity("COG-003"),
                artifact_path: report.to_string_lossy().into(),
            });
        } else if let Ok(rjson) = std::fs::read_to_string(&report) {
            if let Ok(rv) = serde_json::from_str::<serde_json::Value>(&rjson) {
                let fitness = rv.get("fitness").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if (fitness - 1.0).abs() > 1e-9 {
                    diags.push(BreedDiagnostic {
                        law_id: "COG-006",
                        breed_id: bid.clone(),
                        message: format!(
                            "[COG-006] '{bid}' OCEL fitness report claims {fitness:.6} != 1.0"
                        ),
                        severity: severity("COG-006"),
                        artifact_path: report.to_string_lossy().into(),
                    });
                }
                let has_provenance = rv.get("measured_by").is_some()
                    || rv.get("provenance").is_some()
                    || rv.get("run_id").is_some()
                    || rv.get("measured_on").is_some();
                if !has_provenance {
                    diags.push(BreedDiagnostic {
                        law_id: "COG-007",
                        breed_id: bid.clone(),
                        message: format!("[COG-007] '{bid}' report lacks measured-fitness provenance (run_id/measured_by/measured_on) — bare fitness claim is A10"),
                        severity: severity("COG-007"),
                        artifact_path: report.to_string_lossy().into(),
                    });
                }
            }
        }

        // COG-004 / COG-005: Rust fixture missing or missing expected value (A12)
        if !fix_rs.exists() {
            diags.push(BreedDiagnostic {
                law_id: "COG-004",
                breed_id: bid.clone(),
                message: format!("[COG-004] '{bid}' paper fixture missing — tests/fixtures/papers/{bid}.json required (A12 prevention)"),
                severity: severity("COG-004"),
                artifact_path: fix_rs.to_string_lossy().into(),
            });
        } else if let Ok(fjson) = std::fs::read_to_string(&fix_rs) {
            if let Ok(fv) = serde_json::from_str::<serde_json::Value>(&fjson) {
                let has_expected = fv.get("expected").is_some()
                    || fv.get("expected_value").is_some()
                    || fv.get("asserted_value").is_some()
                    || fv.get("paper_value").is_some();
                if !has_expected {
                    diags.push(BreedDiagnostic {
                        law_id: "COG-005",
                        breed_id: bid.clone(),
                        message: format!("[COG-005][A12] '{bid}' fixture cites paper but has no expected.value — citation without assertion is a fraud signal"),
                        severity: severity("COG-005"),
                        artifact_path: fix_rs.to_string_lossy().into(),
                    });
                }
            }
        }

        // COG-008: docs card missing
        if !doc.exists() {
            diags.push(BreedDiagnostic {
                law_id: "COG-008",
                breed_id: bid.clone(),
                message: format!(
                    "[COG-008] '{bid}' docs card missing — docs/breeds/{bid}.md (ceremony row 12)"
                ),
                severity: severity("COG-008"),
                artifact_path: doc.to_string_lossy().into(),
            });
        }

        // COG-009: TS fixture mirror missing
        if !fix_ts.exists() {
            diags.push(BreedDiagnostic {
                law_id: "COG-009",
                breed_id: bid.clone(),
                message: format!("[COG-009] '{bid}' TS fixture mirror missing — packages/cognition/src/__tests__/fixtures/papers/{bid}.json"),
                severity: severity("COG-009"),
                artifact_path: fix_ts.to_string_lossy().into(),
            });
        }

        // COG-010: fresh-name oracle leak into production source (A8)
        if module.exists() {
            if let Ok(source) = std::fs::read_to_string(&module) {
                for name in fresh_name_manifest(&bid) {
                    // Match whole-word occurrences only (exclude test-file paths and comments that
                    // merely document the oracle contract).
                    if source.contains(name) {
                        // Exclude lines that are comments documenting the oracle (// or ///)
                        // A non-comment occurrence is a strong A8 signal.
                        let non_comment_hit = source.lines().any(|line| {
                            let trimmed = line.trim();
                            !trimmed.starts_with("//") && line.contains(name)
                        });
                        if non_comment_hit {
                            diags.push(BreedDiagnostic {
                                law_id: "COG-010",
                                breed_id: bid.clone(),
                                message: format!(
                                    "[COG-010][A8] '{bid}' production source contains oracle fresh-name '{}' — oracle injection fraud signal",
                                    name
                                ),
                                severity: severity("COG-010"),
                                artifact_path: module.to_string_lossy().into(),
                            });
                            break; // one hit per breed is enough to flag
                        }
                    }
                }
            }
        }

        // COG-011: premature PARTIAL_ALIVE (module exists but OCPN or report absent)
        let has_module = module.exists();
        let has_ocpn = ocpn.exists();
        let has_report = report.exists();
        let has_fixture = fix_rs.exists();
        let dod_complete = has_module && has_ocpn && has_report && has_fixture;
        if !dod_complete && has_module {
            // At least one required artifact missing — the registry flip is premature (A10).
            let missing: Vec<&str> = [
                (!has_ocpn).then_some("OCPN"),
                (!has_report).then_some("report"),
                (!has_fixture).then_some("fixture"),
            ]
            .into_iter()
            .flatten()
            .collect();
            if !missing.is_empty() {
                diags.push(BreedDiagnostic {
                    law_id: "COG-011",
                    breed_id: bid.clone(),
                    message: format!(
                        "[COG-011][A10] '{bid}' is PARTIAL_ALIVE but DoD incomplete — missing: {}",
                        missing.join(", ")
                    ),
                    severity: severity("COG-011"),
                    artifact_path: registry_path.to_string_lossy().into(),
                });
            }
        }

        // COG-012: dispatch arm presence — use already-parsed dispatch_src
        if !dispatch_src.is_empty() {
            let arm_present = dispatch_src.contains(&format!("\"{bid}\""))
                || dispatch_src.contains(&format!("BreedId::{}", snake_to_pascal(&bid)));
            if !arm_present {
                diags.push(BreedDiagnostic {
                    law_id: "COG-012",
                    breed_id: bid.clone(),
                    message: format!("[COG-012] '{bid}' has no dispatch arm in src/breeds/dispatch.rs — breed is unreachable at runtime"),
                    severity: severity("COG-012"),
                    artifact_path: dispatch_path.to_string_lossy().into(),
                });
            }
        }
    }

    diags
}

/// Convert snake_case breed id to PascalCase for BreedId enum variant matching.
fn snake_to_pascal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cap_next = true;
    for c in s.chars() {
        if c == '_' {
            cap_next = true;
        } else if cap_next {
            out.extend(c.to_uppercase());
            cap_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Summary counts — used by the release gate check.
#[derive(Debug, Default)]
pub struct AuditSummary {
    /// Number of PARTIAL_ALIVE breeds audited.
    pub total_partial_alive: usize,
    /// Total ERROR-severity diagnostics emitted.
    pub error_count: usize,
    /// Total WARNING-severity diagnostics emitted.
    pub warning_count: usize,
    /// A8 oracle-injection violations (COG-010).
    pub a8_violations: usize,
    /// A10 premature-status-flip violations (COG-011).
    pub a10_violations: usize,
    /// A12 citation-without-assertion violations (COG-005).
    pub a12_violations: usize,
}

impl AuditSummary {
    /// Aggregate diagnostic counts from a completed audit run.
    pub fn from_diagnostics(diags: &[BreedDiagnostic]) -> Self {
        let mut s = AuditSummary::default();
        for d in diags {
            match d.severity {
                DiagnosticSeverity::ERROR => s.error_count += 1,
                DiagnosticSeverity::WARNING => s.warning_count += 1,
                _ => {}
            }
            if d.law_id == "COG-010" {
                s.a8_violations += 1;
            }
            if d.law_id == "COG-011" {
                s.a10_violations += 1;
            }
            if d.law_id == "COG-005" {
                s.a12_violations += 1;
            }
        }
        s
    }

    /// Returns true iff no errors and no A8 violations — the release gate condition.
    pub fn is_release_gate_green(&self) -> bool {
        self.error_count == 0 && self.a8_violations == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_to_pascal_conversions() {
        assert_eq!(snake_to_pascal("ltl_monitor"), "LtlMonitor");
        assert_eq!(snake_to_pascal("bayesian_network"), "BayesianNetwork");
        assert_eq!(snake_to_pascal("asp"), "Asp");
        assert_eq!(snake_to_pascal("meta_reasoning"), "MetaReasoning");
    }

    #[test]
    fn audit_returns_empty_when_no_wasm4pm_root() {
        // A path with no registry.json silently returns no diagnostics.
        let diags = audit_breeds(Path::new("/tmp/nonexistent_workspace_xyz"));
        assert!(diags.is_empty());
    }

    #[test]
    fn dispatch_module_map_derives_non_conventional_locations() {
        // Simulate the dispatch.rs use list that has frame::Eliza and production_rules::Mycin.
        let src = r#"
use crate::breeds::{
    frame::Eliza, frames_inheritance::FramesInheritance,
    production_rules::Mycin,
    cbr::Cbr,
};

fn dispatch(breed: &str, input: &BreedInput) {
    match breed {
        "eliza" => run_breed(&Eliza, input),
        "mycin" => run_breed(&Mycin, input),
        "cbr"   => run_breed(&Cbr, input),
    }
}
"#;
        let map = dispatch_module_map(src);
        assert_eq!(
            map.get("eliza").map(|s| s.as_str()),
            Some("frame"),
            "eliza should derive from frame::Eliza import"
        );
        assert_eq!(
            map.get("mycin").map(|s| s.as_str()),
            Some("production_rules"),
            "mycin should derive from production_rules::Mycin import"
        );
        assert_eq!(
            map.get("cbr").map(|s| s.as_str()),
            Some("cbr"),
            "cbr conventional naming should work"
        );
    }

    #[test]
    fn audit_real_wasm4pm_workspace() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let wasm4pm = super::wasm4pm_root(&root);
        if !wasm4pm
            .join("crates/wasm4pm-cognition/breeds/registry.json")
            .exists()
        {
            // Not in a layout with ../wasm4pm sibling — skip.
            return;
        }
        let diags = audit_breeds(&root);
        // Print for visibility in CI; do not fail on count (this is a live audit, not a fixture).
        for d in &diags {
            eprintln!("{}", d.message);
        }
        let summary = AuditSummary::from_diagnostics(&diags);
        eprintln!(
            "Cognition audit: {} errors, {} warnings, {} A8, {} A10, {} A12",
            summary.error_count,
            summary.warning_count,
            summary.a8_violations,
            summary.a10_violations,
            summary.a12_violations
        );
        // A8 (oracle injection) is always a hard error — no registered breed may have one.
        assert_eq!(
            summary.a8_violations, 0,
            "A8 oracle injection violations detected"
        );
    }
}
