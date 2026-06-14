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

/// Per-server C_D routing source for an attribution decision.
///
/// Records WHICH law-collapse function classified a diagnostic, so that
/// "why is this URI in ANDON" resolves to a specific child server's Λ_CD^(D),
/// not the workspace-wide union. This is the load-bearing distinction for
/// per-server isolation: a `PerServer` route is attributable to one child;
/// a `Union` route is an explicit last-resort with NO server identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AndonRoute {
    /// Classified by the originating server's own prefix set.
    PerServer { server_id: String },
    /// Last-resort: the diagnostic carried no server_id, so the workspace
    /// union governed. This is the ONLY lawful use of the union for ANDON.
    Union,
    /// No automaton matched under the routed C_D — not an ANDON code.
    NotAndon,
}

/// Workspace-wide ANDON prefix registry for diagnostic merge decisions.
///
/// # L7 Speciation — ADMITTED (strict per-server isolation)
///
/// Each project-server entry in `lsp-max.toml` carries an independent law-collapse
/// function Λ_CD^(D). A server that DECLARES its own C_D is classified STRICTLY by it:
/// a `DiagnosticEntry` whose `server_id` has an entry in `server_prefix_overrides` is
/// matched ONLY against that set, so a code declared only by server B does NOT put
/// server A's diagnostic into ANDON — the union is never borrowed for a server with its
/// own laws. A server with NO override has declared no laws of its own and falls back to
/// the workspace baseline (`andon_prefixes`), surfaced as `AndonRoute::Union`; this is
/// how universal receipt laws (`WASM4PM-CROWN-*`) still apply to an unconfigured child.
/// A `None` server_id is the explicit no-identity last resort, also `Union`.
///
/// Status: ADMITTED — `attribute_andon()` returns the routing source; `from_config()`
/// gives EVERY configured server an entry in `server_prefix_overrides` (its own list or
/// the static defaults) via `CompositorConfig::per_server_andon_prefixes()`, so a known
/// server is always classified by its own C_D. The isolation guarantee for configured
/// servers is witnessed by `merge/witness_isolation.rs` (mutation-checked: forcing the
/// configured path to borrow the union fails `cross_server_prefix_does_not_leak_via_union`).
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
    automaton
        .find_iter(code)
        .next()
        .is_some_and(|m| m.start() == 0)
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
            .filter_map(|(id, prefixes)| build_automaton(prefixes).map(|a| (id.clone(), a)))
            .collect();
        Self {
            andon_prefixes,
            andon_automaton,
            server_prefix_overrides,
            server_automatons,
        }
    }

    /// Register a per-server prefix override and rebuild the server's automaton.
    /// Intended for testing and dynamic configuration; prefer `from_config` in production.
    pub fn add_server_prefix_override(&mut self, server_id: String, prefixes: Vec<String>) {
        let automaton = build_automaton(&prefixes);
        self.server_prefix_overrides
            .insert(server_id.clone(), prefixes);
        if let Some(a) = automaton {
            self.server_automatons.insert(server_id, a);
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

    /// Per-server C_D automaton. Returns `None` (NOT the union) when the server
    /// declares no override — strict isolation: a server's diagnostics are
    /// classified only by that server's own Λ_CD^(D).
    fn automaton_for_server(&self, server_id: &str) -> Option<&DoubleArrayAhoCorasick<u32>> {
        self.server_automatons.get(server_id)
    }

    /// Attribute an ANDON classification to a specific routing source.
    ///
    /// Isolation applies to servers that DECLARE a C_D of their own: such a
    /// server is classified ONLY against its own prefix set — a code declared by
    /// server B must not put server A's diagnostic into ANDON, so the union is
    /// never borrowed when the server has its own laws. A server with NO override
    /// has declared no laws of its own and falls back to the workspace baseline
    /// (`Union` route) — this is how universal receipt laws (e.g. `WASM4PM-CROWN-*`)
    /// still apply to an unconfigured child. A `None` server_id is the explicit
    /// no-identity last resort, also governed by the baseline as `Union`.
    pub fn attribute_andon(&self, code: &str, server_id: Option<&str>) -> AndonRoute {
        match server_id {
            // CONFIGURED server: route strictly through its own C_D. Not matching
            // its own set is NotAndon — the union is NOT borrowed. This is the
            // per-server isolation guarantee.
            Some(sid) => match self.automaton_for_server(sid) {
                Some(a) => {
                    if automaton_is_prefix_match(a, code) {
                        AndonRoute::PerServer {
                            server_id: sid.to_string(),
                        }
                    } else {
                        AndonRoute::NotAndon
                    }
                }
                // UNCONFIGURED server (no C_D of its own): fall back to the
                // workspace baseline. Marked `Union` (not `PerServer`) so the
                // attribution stays honest — the baseline governed, not the
                // server's own law. Consistent with `prefixes_for_server`, which
                // already falls back to `andon_prefixes` for unknown servers.
                None => self.baseline_route(code),
            },
            // No server identity: the workspace baseline is the only available C_D.
            None => self.baseline_route(code),
        }
    }

    /// Workspace-baseline ANDON classification, surfaced as `Union` so attribution
    /// records that no per-server C_D governed the decision.
    fn baseline_route(&self, code: &str) -> AndonRoute {
        match self.andon_automaton.as_ref() {
            Some(a) if automaton_is_prefix_match(a, code) => AndonRoute::Union,
            _ => AndonRoute::NotAndon,
        }
    }

    /// O(|code|) ANDON check via daachorse automaton.
    ///
    /// A server with its own C_D is classified strictly by it (no union borrow);
    /// a server with no override, or no server_id at all, falls back to the
    /// workspace baseline. Backed by `attribute_andon`.
    pub fn is_andon_for_server(&self, code: &str, server_id: Option<&str>) -> bool {
        !matches!(self.attribute_andon(code, server_id), AndonRoute::NotAndon)
    }

    pub fn merge(&self, inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)>) -> MergeResult {
        let diagnostics = merge_diagnostics_with_ctx_auto(inputs, self);
        let has_andon_block = diagnostics
            .iter()
            .any(|d| d.severity == 1 && self.is_andon_for_server(&d.code, d.server_id.as_deref()));
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

#[cfg(test)]
mod witness_isolation;
