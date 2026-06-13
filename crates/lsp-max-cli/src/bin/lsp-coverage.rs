//! lsp-coverage — LSP 3.18 spec coverage detector for LanguageServer implementors.
//!
//! Usage:
//!   lsp-coverage --source path/to/main.rs [--json] [--threshold 50] [--skip-max]

use std::collections::BTreeMap;
use std::path::PathBuf;

// ── Method registry ────────────────────────────────────────────────────────

struct MethodEntry {
    name: &'static str,
    category: &'static str,
    is_max: bool,
}

macro_rules! me {
    ($n:expr, $c:expr, $m:expr) => {
        MethodEntry {
            name: $n,
            category: $c,
            is_max: $m,
        }
    };
}

#[rustfmt::skip]
static REGISTRY: &[MethodEntry] = &[
    // Lifecycle
    me!("initialize",                        "Lifecycle",          false),
    me!("initialized",                       "Lifecycle",          false),
    me!("shutdown",                          "Lifecycle",          false),
    // Doc lifecycle
    me!("did_open",                          "Doc lifecycle",      false),
    me!("did_change",                        "Doc lifecycle",      false),
    me!("did_save",                          "Doc lifecycle",      false),
    me!("did_close",                         "Doc lifecycle",      false),
    me!("will_save",                         "Doc lifecycle",      false),
    me!("will_save_wait_until",              "Doc lifecycle",      false),
    // Navigation
    me!("goto_declaration",                  "Navigation",         false),
    me!("goto_definition",                   "Navigation",         false),
    me!("goto_type_definition",              "Navigation",         false),
    me!("goto_implementation",               "Navigation",         false),
    me!("references",                        "Navigation",         false),
    me!("document_highlight",                "Navigation",         false),
    me!("hover",                             "Navigation",         false),
    me!("moniker",                           "Navigation",         false),
    // Call hierarchy
    me!("prepare_call_hierarchy",            "Call hierarchy",     false),
    me!("incoming_calls",                    "Call hierarchy",     false),
    me!("outgoing_calls",                    "Call hierarchy",     false),
    // Type hierarchy
    me!("prepare_type_hierarchy",            "Type hierarchy",     false),
    me!("supertypes",                        "Type hierarchy",     false),
    me!("subtypes",                          "Type hierarchy",     false),
    // Completion
    me!("completion",                        "Completion",         false),
    me!("completion_resolve",                "Completion",         false),
    me!("signature_help",                    "Completion",         false),
    me!("inline_completion",                 "Completion",         false),
    // Symbols
    me!("document_symbol",                   "Symbols",            false),
    me!("symbol",                            "Symbols",            false),
    me!("symbol_resolve",                    "Symbols",            false),
    // Diagnostics
    me!("diagnostic",                        "Diagnostics",        false),
    me!("workspace_diagnostic",              "Diagnostics",        false),
    // Formatting
    me!("formatting",                        "Formatting",         false),
    me!("range_formatting",                  "Formatting",         false),
    me!("ranges_formatting",                 "Formatting",         false),
    me!("on_type_formatting",                "Formatting",         false),
    // Rename
    me!("rename",                            "Rename",             false),
    me!("prepare_rename",                    "Rename",             false),
    // Code Lens
    me!("code_lens",                         "Code Lens",          false),
    me!("code_lens_resolve",                 "Code Lens",          false),
    // Code Action
    me!("code_action",                       "Code Action",        false),
    me!("code_action_resolve",               "Code Action",        false),
    // Semantic Tokens
    me!("semantic_tokens_full",              "Semantic Tokens",    false),
    me!("semantic_tokens_full_delta",        "Semantic Tokens",    false),
    me!("semantic_tokens_range",             "Semantic Tokens",    false),
    // Inlay & Inline
    me!("inlay_hint",                        "Inlay & Inline",     false),
    me!("inlay_hint_resolve",                "Inlay & Inline",     false),
    me!("inline_value",                      "Inlay & Inline",     false),
    // Document Links
    me!("document_link",                     "Document Links",     false),
    me!("document_link_resolve",             "Document Links",     false),
    // Folding & Selection
    me!("folding_range",                     "Folding & Selection",false),
    me!("selection_range",                   "Folding & Selection",false),
    me!("linked_editing_range",              "Folding & Selection",false),
    // Color
    me!("document_color",                    "Color",              false),
    me!("color_presentation",                "Color",              false),
    // Notebook
    me!("did_open_notebook_document",        "Notebook",           false),
    me!("did_change_notebook_document",      "Notebook",           false),
    me!("did_save_notebook_document",        "Notebook",           false),
    me!("did_close_notebook_document",       "Notebook",           false),
    // File operations
    me!("will_create_files",                 "File operations",    false),
    me!("did_create_files",                  "File operations",    false),
    me!("will_rename_files",                 "File operations",    false),
    me!("did_rename_files",                  "File operations",    false),
    me!("will_delete_files",                 "File operations",    false),
    me!("did_delete_files",                  "File operations",    false),
    // Workspace
    me!("did_change_configuration",          "Workspace",          false),
    me!("did_change_workspace_folders",      "Workspace",          false),
    me!("did_change_watched_files",          "Workspace",          false),
    me!("execute_command",                   "Workspace",          false),
    me!("text_document_content",             "Workspace",          false),
    me!("work_done_progress_cancel",         "Workspace",          false),
    me!("set_trace",                         "Workspace",          false),
    me!("progress",                          "Workspace",          false),
    // max/* custom
    me!("max_snapshot",                      "max/* custom",       true),
    me!("max_conformance_vector",            "max/* custom",       true),
    me!("max_explain_diagnostic",            "max/* custom",       true),
    me!("max_receipt",                       "max/* custom",       true),
    me!("max_run_gate",                      "max/* custom",       true),
    me!("max_repair_plan",                   "max/* custom",       true),
    me!("max_apply_repair_transaction",      "max/* custom",       true),
    me!("max_export_analysis_bundle",        "max/* custom",       true),
    me!("max_clear_diagnostic",              "max/* custom",       true),
    me!("max_release_actuation",             "max/* custom",       true),
    me!("max_admission",                     "max/* custom",       true),
    me!("max_autonomic_loop",                "max/* custom",       true),
    me!("max_chain",                         "max/* custom",       true),
    me!("max_hook",                          "max/* custom",       true),
    me!("max_hook_graph",                    "max/* custom",       true),
    me!("max_lawful_transition",             "max/* custom",       true),
    me!("max_ledger_report",                 "max/* custom",       true),
    me!("max_manifold_snapshot",             "max/* custom",       true),
    me!("max_propagate",                     "max/* custom",       true),
    me!("max_refusal",                       "max/* custom",       true),
    me!("max_replay",                        "max/* custom",       true),
    me!("max_verify_ledger",                 "max/* custom",       true),
    me!("max_conformance_delta",             "max/* custom",       true),
    me!("max_dump_state",                    "max/* custom",       true),
    me!("max_restore_state",                 "max/* custom",       true),
    me!("max_instance_list",                 "max/* custom",       true),
    me!("max_reset",                         "max/* custom",       true),
    me!("max_lsif",                          "max/* custom",       true),
];

// ── Stub patterns ──────────────────────────────────────────────────────────

// These body signatures (trimmed) indicate a trivial stub.
static STUB_BODIES: &[&str] = &[
    "Ok(None)",
    "Ok(())",
    "Ok(Default::default())",
    "Default::default()",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
];

// ── Detection ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Implemented,
    Stub,
    Missing,
}

fn extract_body_after(source: &str, fn_sig_end: usize) -> Option<String> {
    let rest = &source[fn_sig_end..];
    // skip to opening brace
    let open = rest.find('{')?;
    let body_start = fn_sig_end + open + 1;
    let mut depth = 1usize;
    let chars: Vec<char> = source[body_start..].chars().collect();
    let mut i = 0;
    while i < chars.len() && depth > 0 {
        match chars[i] {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    Some(
        source[body_start..body_start + i.saturating_sub(1)]
            .trim()
            .to_string(),
    )
}

fn classify(source: &str, method: &str) -> Status {
    // Look for `async fn method_name` — must be a trait override
    let pattern = format!("async fn {method}");
    let Some(pos) = source.find(&pattern) else {
        return Status::Missing;
    };
    // Find the body
    let sig_end = pos + pattern.len();
    let Some(body) = extract_body_after(source, sig_end) else {
        return Status::Missing;
    };
    // Single-line body check for stubs
    if !body.contains('\n') || body.lines().count() <= 2 {
        let trimmed = body.trim();
        for stub in STUB_BODIES {
            if trimmed.starts_with(stub) {
                return Status::Stub;
            }
        }
        // Returning params or empty
        if trimmed.is_empty() || trimmed == "params" {
            return Status::Stub;
        }
    }
    Status::Implemented
}

// ── CLI ────────────────────────────────────────────────────────────────────

fn usage() -> ! {
    eprintln!("Usage: lsp-coverage --source <path.rs> [--json] [--threshold <0-100>] [--skip-max]");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut source_path: Option<PathBuf> = None;
    let mut json_mode = false;
    let mut threshold: Option<u32> = None;
    let mut skip_max = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source" => {
                i += 1;
                source_path = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            "--json" => json_mode = true,
            "--skip-max" => skip_max = true,
            "--threshold" => {
                i += 1;
                threshold = Some(
                    args.get(i)
                        .unwrap_or_else(|| usage())
                        .parse()
                        .unwrap_or_else(|_| usage()),
                );
            }
            _ => {}
        }
        i += 1;
    }

    let path = source_path.unwrap_or_else(|| usage());
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", path.display());
        std::process::exit(2);
    });

    // Classify every method
    let results: Vec<(&MethodEntry, Status)> = REGISTRY
        .iter()
        .filter(|e| !(skip_max && e.is_max))
        .map(|e| (e, classify(&source, e.name)))
        .collect();

    if json_mode {
        print_json(&path.display().to_string(), &results);
    } else {
        print_table(&path.display().to_string(), &results);
    }

    // Exit code for CI gate
    if let Some(t) = threshold {
        let standard: Vec<_> = results.iter().filter(|(e, _)| !e.is_max).collect();
        let impl_count = standard
            .iter()
            .filter(|(_, s)| *s == Status::Implemented)
            .count();
        let pct = if standard.is_empty() {
            100u32
        } else {
            (impl_count * 100 / standard.len()) as u32
        };
        if pct < t {
            std::process::exit(1);
        }
    }
}

// ── Output ─────────────────────────────────────────────────────────────────

fn print_table(label: &str, results: &[(&MethodEntry, Status)]) {
    println!("lsp-coverage: {label}");
    println!("LSP 3.18 Coverage Report");
    println!("{}", "═".repeat(62));
    println!(
        "{:<22} {:>5}  {:>4}  {:>4}  {:>7}  {:>4}",
        "Category", "Total", "Impl", "Stub", "Missing", "%"
    );
    println!("{}", "─".repeat(62));

    // Group by category preserving insertion order via BTreeMap
    let mut by_cat: BTreeMap<&str, Vec<&(&MethodEntry, Status)>> = BTreeMap::new();
    for r in results {
        by_cat.entry(r.0.category).or_default().push(r);
    }

    let mut std_total = 0usize;
    let mut std_impl = 0usize;
    let mut std_stub = 0usize;
    let mut std_miss = 0usize;
    let mut max_total = 0usize;
    let mut max_impl = 0usize;
    let mut max_stub = 0usize;
    let mut max_miss = 0usize;

    for (cat, entries) in &by_cat {
        let total = entries.len();
        let impl_c = entries
            .iter()
            .filter(|(_, s)| *s == Status::Implemented)
            .count();
        let stub_c = entries.iter().filter(|(_, s)| *s == Status::Stub).count();
        let miss_c = entries
            .iter()
            .filter(|(_, s)| *s == Status::Missing)
            .count();
        let pct = (impl_c * 100).checked_div(total).unwrap_or(0);
        println!(
            "{:<22} {:>5}  {:>4}  {:>4}  {:>7}  {:>3}%",
            cat, total, impl_c, stub_c, miss_c, pct
        );

        let is_max = entries.first().map(|(e, _)| e.is_max).unwrap_or(false);
        if is_max {
            max_total += total;
            max_impl += impl_c;
            max_stub += stub_c;
            max_miss += miss_c;
        } else {
            std_total += total;
            std_impl += impl_c;
            std_stub += stub_c;
            std_miss += miss_c;
        }
    }

    println!("{}", "─".repeat(62));
    let std_pct = (std_impl * 100).checked_div(std_total).unwrap_or(0);
    let max_pct = (max_impl * 100).checked_div(max_total).unwrap_or(0);
    let all_t = std_total + max_total;
    let all_i = std_impl + max_impl;
    let all_pct = (all_i * 100).checked_div(all_t).unwrap_or(0);
    println!(
        "{:<22} {:>5}  {:>4}  {:>4}  {:>7}  {:>3}%",
        "STANDARD LSP 3.18", std_total, std_impl, std_stub, std_miss, std_pct
    );
    println!(
        "{:<22} {:>5}  {:>4}  {:>4}  {:>7}  {:>3}%",
        "max/* custom", max_total, max_impl, max_stub, max_miss, max_pct
    );
    println!("{}", "─".repeat(62));
    println!(
        "{:<22} {:>5}  {:>4}  {:>4}  {:>7}  {:>3}%",
        "TOTAL",
        all_t,
        all_i,
        std_stub + max_stub,
        std_miss + max_miss,
        all_pct
    );

    // Missing methods by category
    let missing: Vec<_> = results
        .iter()
        .filter(|(_, s)| *s == Status::Missing)
        .collect();
    let missing_std: BTreeMap<&str, Vec<&str>> = {
        let mut m: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (e, _s) in results
            .iter()
            .filter(|(e, s)| *s == Status::Missing && !e.is_max)
        {
            m.entry(e.category).or_default().push(e.name);
        }
        m
    };
    let _ = missing; // suppress unused warning

    if !missing_std.is_empty() {
        println!("\nMissing (standard LSP 3.18):");
        for (cat, names) in &missing_std {
            println!("  [{cat:<20}]  {}", names.join(", "));
        }
    }

    // Stub methods
    let stubs: Vec<&str> = results
        .iter()
        .filter(|(e, s)| *s == Status::Stub && !e.is_max)
        .map(|(e, _)| e.name)
        .collect();
    if !stubs.is_empty() {
        println!("\nStubs (trivial bodies):");
        println!("  {}", stubs.join(", "));
    }

    // Missing max/*
    let missing_max: Vec<&str> = results
        .iter()
        .filter(|(e, s)| *s == Status::Missing && e.is_max)
        .map(|(e, _)| e.name)
        .collect();
    if !missing_max.is_empty() {
        println!("\nUnimplemented max/* ({} methods):", missing_max.len());
        println!("  {}", missing_max.join(", "));
    }
}

fn print_json(label: &str, results: &[(&MethodEntry, Status)]) {
    let std_total = results.iter().filter(|(e, _)| !e.is_max).count();
    let std_impl = results
        .iter()
        .filter(|(e, s)| !e.is_max && *s == Status::Implemented)
        .count();
    let std_stub = results
        .iter()
        .filter(|(e, s)| !e.is_max && *s == Status::Stub)
        .count();
    let std_miss = results
        .iter()
        .filter(|(e, s)| !e.is_max && *s == Status::Missing)
        .count();
    let max_total = results.iter().filter(|(e, _)| e.is_max).count();
    let max_impl = results
        .iter()
        .filter(|(e, s)| e.is_max && *s == Status::Implemented)
        .count();

    println!("{{");
    println!("  \"source\": \"{label}\",");
    println!("  \"standard\": {{ \"total\": {std_total}, \"implemented\": {std_impl}, \"stub\": {std_stub}, \"missing\": {std_miss}, \"pct\": {} }},", (std_impl * 100).checked_div(std_total).unwrap_or(0));
    println!("  \"max_custom\": {{ \"total\": {max_total}, \"implemented\": {max_impl} }},");
    println!("  \"methods\": [");
    for (i, (e, s)) in results.iter().enumerate() {
        let status = match s {
            Status::Implemented => "implemented",
            Status::Stub => "stub",
            Status::Missing => "missing",
        };
        let comma = if i + 1 < results.len() { "," } else { "" };
        println!("    {{ \"name\": \"{}\", \"category\": \"{}\", \"is_max\": {}, \"status\": \"{status}\" }}{comma}", e.name, e.category, e.is_max);
    }
    println!("  ]");
    println!("}}");
}
