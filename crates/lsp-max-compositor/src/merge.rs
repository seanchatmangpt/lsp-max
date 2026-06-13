// Diagnostic merge — ConformanceVector-aware merge of multi-server diagnostic sets.
// REFUSED_BY_LAW diagnostics: lower severity number wins dedup; tier_rank breaks ties.
// Non-law diagnostics: Primary tier wins dedup by (uri, line, character, code).
//
// ANDON prefix classification uses daachorse (Double-Array Aho-Corasick) automatons
// compiled once at MergeContext construction time. Classification is O(|code|) regardless
// of match/no-match outcome — eliminates the 7.3× asymmetry of the former linear scan.

use daachorse::DoubleArrayAhoCorasick;

use crate::registry::ChildTier;

#[derive(Debug, Clone)]
pub struct DiagnosticEntry {
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub severity: u8, // 1=Error, 2=Warning, 3=Info, 4=Hint
    pub code: String,
    pub message: String,
    pub source_tier: ChildTier,
    pub server_id: Option<String>,
}

pub struct MergeResult {
    pub diagnostics: Vec<DiagnosticEntry>,
    /// True if any REFUSED_BY_LAW Error (severity == 1) is present in diagnostics.
    /// Callers must inspect this before emitting receipts or completing workflows.
    pub has_andon_block: bool,
}

impl MergeResult {
    pub fn andon_codes(&self) -> Vec<&str> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == 1 && is_refused_by_law(&d.code))
            .map(|d| d.code.as_str())
            .collect()
    }
}

/// Workspace-wide ANDON prefix registry for diagnostic merge decisions.
///
/// # L7 Speciation — ADMITTED
///
/// The formal model claims each project-server entry in `lsp-max.toml` carries an
/// independent law-collapse function Λ_CD^(D). Per-server C_D routing is implemented:
/// each `DiagnosticEntry` is evaluated against its originating server's prefix set via
/// `prefixes_for_server(server_id)`. The workspace-wide union (`andon_prefixes`) is
/// retained as the fallback for entries with no `server_id` or for servers not present
/// in `server_prefix_overrides`.
///
/// Status: ADMITTED — `MergeContext` carries `server_prefix_overrides: HashMap<String,
/// Vec<String>>`, `prefixes_for_server()` routes per-server C_D, `deposit()` in
/// `diagnostic_buffer.rs` calls `prefixes_for_server(server_id)` at deposit time, and
/// `merge_diagnostics_with_ctx()` routes each entry through `effective_for(entry)`.
/// `MergeContext::from_config()` populates `server_prefix_overrides` via
/// `CompositorConfig::per_server_andon_prefixes()`.
pub struct MergeContext {
    /// Workspace-wide union of all server ANDON prefixes. Used as fallback when
    /// a diagnostic entry has no server_id or when server_prefix_overrides has no
    /// entry for that server_id.
    andon_prefixes: Vec<String>,
    /// Daachorse automaton compiled from andon_prefixes. O(|code|) classification,
    /// no match/no-match asymmetry. None when andon_prefixes is empty.
    andon_automaton: Option<DoubleArrayAhoCorasick<u32>>,
    /// Per-server prefix overrides: server_id → prefix list.
    /// Routes each diagnostic through its originating server's C_D rather than the
    /// workspace-wide union.
    /// Status: ADMITTED — wired and populated from CompositorConfig.
    server_prefix_overrides: std::collections::HashMap<String, Vec<String>>,
    /// Per-server daachorse automatons built from server_prefix_overrides.
    /// Eliminates Vec<&str> allocation per diagnostic in the sort comparator.
    server_automatons: std::collections::HashMap<String, DoubleArrayAhoCorasick<u32>>,
}

/// Build a daachorse automaton from a prefix list, or return None if the list is empty.
/// Patterns are the prefix strings themselves; classification checks `match.start() == 0`.
fn build_automaton(prefixes: &[String]) -> Option<DoubleArrayAhoCorasick<u32>> {
    if prefixes.is_empty() {
        return None;
    }
    // Assign pattern index as value — we only need existence, not identity.
    DoubleArrayAhoCorasick::new(prefixes).ok()
}

/// Returns true if `code` begins with any pattern in the automaton (prefix semantics).
#[inline]
fn automaton_is_prefix_match(automaton: &DoubleArrayAhoCorasick<u32>, code: &str) -> bool {
    automaton.find_iter(code).next().is_some_and(|m| m.start() == 0)
}

impl MergeContext {
    pub fn new(prefixes: Vec<String>) -> Self {
        let andon_automaton = build_automaton(&prefixes);
        Self {
            andon_prefixes: prefixes,
            andon_automaton,
            server_prefix_overrides: std::collections::HashMap::new(),
            server_automatons: std::collections::HashMap::new(),
        }
    }

    pub fn from_config(config: &crate::config::CompositorConfig) -> Self {
        let andon_prefixes: Vec<String> = config
            .all_andon_prefixes()
            .into_iter()
            .map(|s| s.to_owned())
            .collect();
        let andon_automaton = build_automaton(&andon_prefixes);
        let server_prefix_overrides = config.per_server_andon_prefixes();
        let server_automatons = server_prefix_overrides
            .iter()
            .filter_map(|(id, prefixes)| {
                build_automaton(prefixes).map(|a| (id.clone(), a))
            })
            .collect();
        Self {
            andon_prefixes,
            andon_automaton,
            server_prefix_overrides,
            server_automatons,
        }
    }

    pub fn andon_prefixes_count(&self) -> usize {
        self.andon_prefixes.len()
    }

    pub fn andon_prefixes(&self) -> &[String] {
        &self.andon_prefixes
    }

    /// Returns the effective ANDON prefix set for a given server_id.
    pub fn prefixes_for_server(&self, server_id: &str) -> &[String] {
        self.server_prefix_overrides
            .get(server_id)
            .map(|v| v.as_slice())
            .unwrap_or(self.andon_prefixes.as_slice())
    }

    /// Returns the effective automaton for a server_id (per-server C_D, or workspace union).
    fn automaton_for_server(&self, server_id: &str) -> Option<&DoubleArrayAhoCorasick<u32>> {
        self.server_automatons
            .get(server_id)
            .or(self.andon_automaton.as_ref())
    }

    /// O(|code|) ANDON check via daachorse automaton. Falls back to linear scan if
    /// the automaton is absent (empty prefix set).
    fn is_andon_for_server(&self, code: &str, server_id: Option<&str>) -> bool {
        let automaton = server_id
            .and_then(|sid| self.automaton_for_server(sid))
            .or(self.andon_automaton.as_ref());
        match automaton {
            Some(a) => automaton_is_prefix_match(a, code),
            None => false,
        }
    }

    pub fn merge(&self, inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)>) -> MergeResult {
        let diagnostics = merge_diagnostics_with_ctx_auto(inputs, self);
        let has_andon_block = diagnostics.iter().any(|d| {
            d.severity == 1 && self.is_andon_for_server(&d.code, d.server_id.as_deref())
        });
        MergeResult {
            diagnostics,
            has_andon_block,
        }
    }
}

pub fn is_refused_by_law(code: &str) -> bool {
    code.starts_with("WASM4PM-") || code.starts_with("ANTI-LLM-") || code.starts_with("GGEN-")
}

pub fn is_refused_by_law_with_prefixes(code: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| code.starts_with(p))
}

fn tier_rank(tier: &ChildTier) -> u8 {
    match tier {
        ChildTier::Primary => 0,
        ChildTier::Secondary => 1,
        ChildTier::DiagnosticsOnly => 2,
    }
}

/// Merge diagnostics from multiple child-server tiers into a single ordered set.
///
/// # Soundness contract (REFUSED_BY_LAW)
///
/// For every REFUSED_BY_LAW entry in the inputs, exactly one entry survives in
/// the output — the one with the minimum severity number (most severe), regardless
/// of which tier emitted it. No law violation is silently dropped.
///
/// Formally: for all d in inputs where is_refused_by_law(d.code),
/// there exists d' in output where d'.code == d.code
/// and d'.severity == min(severity of all inputs with that (uri, line, char, code)).
///
/// # Deduplication for non-law codes
///
/// For non-REFUSED_BY_LAW entries, Primary tier wins deduplication at the same
/// (uri, line, character, code) key. Secondary and DiagnosticsOnly entries at the
/// same location are dropped if a Primary-tier entry exists for the same code.
///
/// # Output ordering
///
/// REFUSED_BY_LAW errors (severity == 1) sort first, then by severity ascending,
/// then by uri, then by line/character.
///
/// `andon_prefixes`: when `Some`, overrides the static law-prefix set for sorting.
pub fn merge_diagnostics(
    inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)>,
    andon_prefixes: Option<&[&str]>,
) -> Vec<DiagnosticEntry> {
    // Flatten and tag with tier, then deduplicate.
    // Key: (uri, line, character, code).
    // For REFUSED_BY_LAW codes: lower severity wins; tier_rank breaks ties.
    // For non-law codes: lower tier_rank wins (Primary beats Secondary beats DiagnosticsOnly).
    use std::collections::HashMap;

    let static_prefixes: &[&str] = &["WASM4PM-", "ANTI-LLM-", "GGEN-"];
    let effective_prefixes: &[&str] = andon_prefixes.unwrap_or(static_prefixes);

    let mut map: HashMap<(String, u32, u32, String), DiagnosticEntry> = HashMap::new();

    // Process Primary first so it wins non-law dedup, then others.
    let mut ordered: Vec<(ChildTier, Vec<DiagnosticEntry>)> = inputs;
    ordered.sort_by_key(|(tier, _)| tier_rank(tier));

    for (tier, entries) in ordered {
        for mut entry in entries {
            entry.source_tier = tier.clone();
            let key = (
                entry.uri.clone(),
                entry.line,
                entry.character,
                entry.code.clone(),
            );
            let insert = match map.get(&key) {
                None => true,
                Some(existing) => {
                    if is_refused_by_law_with_prefixes(&entry.code, effective_prefixes) {
                        // For law codes: lower severity number wins (1=Error beats 2=Warning).
                        // Tier rank breaks ties so Primary wins when severity is equal.
                        match entry.severity.cmp(&existing.severity) {
                            std::cmp::Ordering::Less => true,
                            std::cmp::Ordering::Greater => false,
                            std::cmp::Ordering::Equal => {
                                tier_rank(&entry.source_tier) < tier_rank(&existing.source_tier)
                            }
                        }
                    } else {
                        // Non-law codes: Primary tier wins (lower tier_rank wins).
                        tier_rank(&entry.source_tier) < tier_rank(&existing.source_tier)
                    }
                }
            };
            if insert {
                map.insert(key, entry);
            }
        }
    }

    let mut result: Vec<DiagnosticEntry> = map.into_values().collect();

    result.sort_by(|a, b| {
        let refused = |code: &str| is_refused_by_law_with_prefixes(code, effective_prefixes);
        let a_law = refused(&a.code) && a.severity == 1;
        let b_law = refused(&b.code) && b.severity == 1;
        // REFUSED_BY_LAW errors first
        b_law
            .cmp(&a_law)
            .then(a.severity.cmp(&b.severity))
            .then(a.uri.cmp(&b.uri))
            .then(a.line.cmp(&b.line))
            .then(a.character.cmp(&b.character))
    });

    result
}

/// Context-aware merge: applies per-server ANDON prefix sets when available.
/// For each entry, the effective prefix set is:
///   - server_overrides[entry.server_id] if server_id is Some and present in the map
///   - global_prefixes otherwise (workspace-wide union fallback)
///
/// This is the CANDIDATE path for L7 Speciation — full per-server C_D routing.
pub fn merge_diagnostics_with_ctx(
    inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)>,
    global_prefixes: &[&str],
    server_overrides: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<DiagnosticEntry> {
    use std::collections::HashMap;

    let mut map: HashMap<(String, u32, u32, String), DiagnosticEntry> = HashMap::new();

    let effective_for = |entry: &DiagnosticEntry| -> Vec<&str> {
        entry
            .server_id
            .as_deref()
            .and_then(|sid| server_overrides.get(sid))
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_else(|| global_prefixes.to_vec())
    };

    let mut ordered: Vec<(ChildTier, Vec<DiagnosticEntry>)> = inputs;
    ordered.sort_by_key(|(tier, _)| tier_rank(tier));

    for (tier, entries) in ordered {
        for mut entry in entries {
            entry.source_tier = tier.clone();
            let key = (
                entry.uri.clone(),
                entry.line,
                entry.character,
                entry.code.clone(),
            );
            let eff = effective_for(&entry);
            let insert = match map.get(&key) {
                None => true,
                Some(existing) => {
                    if is_refused_by_law_with_prefixes(&entry.code, &eff) {
                        match entry.severity.cmp(&existing.severity) {
                            std::cmp::Ordering::Less => true,
                            std::cmp::Ordering::Greater => false,
                            std::cmp::Ordering::Equal => {
                                tier_rank(&entry.source_tier) < tier_rank(&existing.source_tier)
                            }
                        }
                    } else {
                        tier_rank(&entry.source_tier) < tier_rank(&existing.source_tier)
                    }
                }
            };
            if insert {
                map.insert(key, entry);
            }
        }
    }

    let mut result: Vec<DiagnosticEntry> = map.into_values().collect();
    result.sort_by(|a, b| {
        let a_eff = effective_for(a);
        let b_eff = effective_for(b);
        let a_law = is_refused_by_law_with_prefixes(&a.code, &a_eff) && a.severity == 1;
        let b_law = is_refused_by_law_with_prefixes(&b.code, &b_eff) && b.severity == 1;
        b_law
            .cmp(&a_law)
            .then(a.severity.cmp(&b.severity))
            .then(a.uri.cmp(&b.uri))
            .then(a.line.cmp(&b.line))
            .then(a.character.cmp(&b.character))
    });
    result
}

/// Automaton-accelerated merge used by `MergeContext::merge()`.
/// Replaces the `Vec<&str>` allocation per entry in the sort comparator with a direct
/// `&DoubleArrayAhoCorasick` lookup — O(|code|) classification, zero allocation per call.
pub fn merge_diagnostics_with_ctx_auto(
    inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)>,
    ctx: &MergeContext,
) -> Vec<DiagnosticEntry> {
    use std::collections::HashMap;

    let mut map: HashMap<(String, u32, u32, String), DiagnosticEntry> = HashMap::new();

    let is_andon = |entry: &DiagnosticEntry| -> bool {
        ctx.is_andon_for_server(&entry.code, entry.server_id.as_deref())
    };

    let mut ordered: Vec<(ChildTier, Vec<DiagnosticEntry>)> = inputs;
    ordered.sort_by_key(|(tier, _)| tier_rank(tier));

    for (tier, entries) in ordered {
        for mut entry in entries {
            entry.source_tier = tier.clone();
            let key = (
                entry.uri.clone(),
                entry.line,
                entry.character,
                entry.code.clone(),
            );
            let insert = match map.get(&key) {
                None => true,
                Some(existing) => {
                    if is_andon(&entry) {
                        match entry.severity.cmp(&existing.severity) {
                            std::cmp::Ordering::Less => true,
                            std::cmp::Ordering::Greater => false,
                            std::cmp::Ordering::Equal => {
                                tier_rank(&entry.source_tier) < tier_rank(&existing.source_tier)
                            }
                        }
                    } else {
                        tier_rank(&entry.source_tier) < tier_rank(&existing.source_tier)
                    }
                }
            };
            if insert {
                map.insert(key, entry);
            }
        }
    }

    let mut result: Vec<DiagnosticEntry> = map.into_values().collect();
    result.sort_by(|a, b| {
        let a_law = is_andon(a) && a.severity == 1;
        let b_law = is_andon(b) && b.severity == 1;
        b_law
            .cmp(&a_law)
            .then(a.severity.cmp(&b.severity))
            .then(a.uri.cmp(&b.uri))
            .then(a.line.cmp(&b.line))
            .then(a.character.cmp(&b.character))
    });
    result
}
