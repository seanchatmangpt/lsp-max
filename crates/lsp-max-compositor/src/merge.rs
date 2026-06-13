// Diagnostic merge — ConformanceVector-aware merge of multi-server diagnostic sets.
// REFUSED_BY_LAW diagnostics: lower severity number wins dedup; tier_rank breaks ties.
// Non-law diagnostics: Primary tier wins dedup by (uri, line, character, code).

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
/// # L7 Speciation — PARTIAL
///
/// The formal model claims each project-server entry in `lsp-max.toml` carries an
/// independent law-collapse function Λ_CD^(D). In the current implementation, per-server
/// `andon_code_prefixes` lists are aggregated into a single workspace-wide union at
/// construction time (via `CompositorConfig::all_andon_prefixes()`). Merge evaluation
/// then tests every diagnostic against this union, not against the originating server's
/// individual prefix set.
///
/// Status: PARTIAL — the union is a superset of every individual Λ_CD^(D), so the
/// implementation is more restrictive than the formal claim (no law violation escapes),
/// but the per-server isolation the formal model describes is not enforced. A diagnostic
/// code from server A will trigger ANDON even if only server B declared that prefix.
///
/// Next step to ADMIT: route each `DiagnosticEntry` through the originating server's
/// prefix set at merge time rather than testing against the constructed union. This
/// requires `MergeContext` to carry a `HashMap<server_id, Vec<String>>` and
/// `merge_diagnostics` to receive the per-entry server identity.
pub struct MergeContext {
    /// Workspace-wide union of all server ANDON prefixes. Used as fallback when
    /// a diagnostic entry has no server_id or when server_prefix_overrides has no
    /// entry for that server_id.
    andon_prefixes: Vec<String>,
    /// Per-server prefix overrides: server_id → prefix list.
    /// When populated (via `from_config`), merge evaluation routes each diagnostic
    /// through its originating server's C_D rather than the workspace-wide union.
    /// Status: CANDIDATE — wired but only populated when CompositorConfig is available.
    server_prefix_overrides: std::collections::HashMap<String, Vec<String>>,
}

impl MergeContext {
    pub fn new(prefixes: Vec<String>) -> Self {
        Self {
            andon_prefixes: prefixes,
            server_prefix_overrides: std::collections::HashMap::new(),
        }
    }

    pub fn from_config(config: &crate::config::CompositorConfig) -> Self {
        Self {
            andon_prefixes: config
                .all_andon_prefixes()
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
            server_prefix_overrides: config.per_server_andon_prefixes(),
        }
    }

    pub fn andon_prefixes_count(&self) -> usize {
        self.andon_prefixes.len()
    }

    pub fn andon_prefixes(&self) -> &[String] {
        &self.andon_prefixes
    }

    /// Returns the effective ANDON prefix set for a given server_id.
    /// If the server has a per-server override, returns that; otherwise returns
    /// the workspace-wide union (fallback for servers not in lsp-max.toml).
    pub fn prefixes_for_server(&self, server_id: &str) -> &[String] {
        self.server_prefix_overrides
            .get(server_id)
            .map(|v| v.as_slice())
            .unwrap_or(self.andon_prefixes.as_slice())
    }

    pub fn merge(&self, inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)>) -> MergeResult {
        let refs: Vec<&str> = self.andon_prefixes.iter().map(|s| s.as_str()).collect();
        let diagnostics = merge_diagnostics_with_ctx(inputs, &refs, &self.server_prefix_overrides);
        let has_andon_block = diagnostics.iter().any(|d| {
            if d.severity != 1 {
                return false;
            }
            let effective: Vec<&str> = d
                .server_id
                .as_deref()
                .and_then(|sid| self.server_prefix_overrides.get(sid))
                .map(|v| v.iter().map(|s| s.as_str()).collect())
                .unwrap_or_else(|| refs.clone());
            is_refused_by_law_with_prefixes(&d.code, &effective)
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
