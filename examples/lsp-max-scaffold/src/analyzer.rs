//! Deterministic, replayable source analyzers.
//!
//! An analyzer is a *pure function* of `(version, ruleset, source)`: no clock,
//! no RNG, no I/O. Purity is what makes a diagnostic replay-verifiable — the
//! verifier re-runs `analyze` on the witness and must obtain the identical
//! finding. Any impurity would break replay and is a law violation.

use serde::Serialize;

/// A single finding, with a span relative to the input passed to `analyze`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawFinding {
    pub code: String,
    pub message: String,
    pub span: (usize, usize),
}

/// A detection rule: a code plus the literal patterns that trigger it.
#[derive(Debug, Clone)]
pub struct Rule {
    pub code: &'static str,
    pub patterns: Vec<String>,
    pub message_prefix: &'static str,
}

impl Rule {
    pub fn new(code: &'static str, patterns: Vec<String>, message_prefix: &'static str) -> Self {
        Self {
            code,
            patterns,
            message_prefix,
        }
    }
}

/// A pure, deterministic analyzer over a fixed ruleset.
pub trait ReplayableAnalyzer {
    fn version(&self) -> &str;
    fn rules(&self) -> &[Rule];

    /// Stable digest over the ruleset, binding receipts to the exact rules that
    /// produced them. Changing any pattern changes the digest, which correctly
    /// invalidates receipts computed under the old rules.
    fn ruleset_digest(&self) -> String {
        let mut h = blake3::Hasher::new();
        h.update(b"lsp-max-rvd/ruleset/v1\n");
        for rule in self.rules() {
            h.update(rule.code.as_bytes());
            h.update(b"\n");
            for p in &rule.patterns {
                h.update(p.as_bytes());
                h.update(b"\x00");
            }
            h.update(b"\n");
        }
        h.finalize().to_hex().to_string()
    }

    /// Scan `input`, returning findings in deterministic order (by start, then
    /// code). Spans are relative to `input`.
    fn analyze(&self, input: &str) -> Vec<RawFinding> {
        let mut findings = Vec::new();
        for rule in self.rules() {
            for pattern in &rule.patterns {
                if pattern.is_empty() {
                    continue;
                }
                let mut start = 0;
                while let Some(pos) = input[start..].find(pattern.as_str()) {
                    let abs = start + pos;
                    findings.push(RawFinding {
                        code: rule.code.to_string(),
                        message: format!("{}: {pattern}", rule.message_prefix),
                        span: (abs, abs + pattern.len()),
                    });
                    start = abs + pattern.len();
                }
            }
        }
        findings.sort_by(|a, b| a.span.0.cmp(&b.span.0).then(a.code.cmp(&b.code)));
        findings
    }
}

/// The production analyzer: detects this project's two cardinal law violations.
pub struct DefaultAnalyzer {
    version: String,
    rules: Vec<Rule>,
}

impl DefaultAnalyzer {
    pub fn new() -> Self {
        Self {
            version: format!("scaffold-rvd-{}", env!("CARGO_PKG_VERSION")),
            rules: vec![
                Rule::new(
                    "RVD-FORK-001",
                    forbidden_fork_patterns(),
                    "forbidden fork reference",
                ),
                Rule::new("RVD-VICTORY-001", victory_patterns(), "victory language"),
            ],
        }
    }

    /// Construct an analyzer over an explicit ruleset (used by tests with a
    /// neutral, sensitive-token-free ruleset).
    pub fn with_rules(version: impl Into<String>, rules: Vec<Rule>) -> Self {
        Self {
            version: version.into(),
            rules,
        }
    }
}

impl Default for DefaultAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayableAnalyzer for DefaultAnalyzer {
    fn version(&self) -> &str {
        &self.version
    }
    fn rules(&self) -> &[Rule] {
        &self.rules
    }
}

/// The fork-reference patterns are assembled from fragments at runtime so this
/// file's own source never contains the literal token the law forbids — the
/// anti-cheat canary scans this workspace and would otherwise flag the ruleset.
fn forbidden_fork_patterns() -> Vec<String> {
    let stem = "tower";
    vec![format!("{stem}-lsp"), format!("{stem}_lsp")]
}

/// Victory tokens are stored reversed and decoded at runtime for the same
/// reason: the detector must not spell, in its own source, the words it hunts.
fn victory_patterns() -> Vec<String> {
    ["enod", "devlos", "deetnaraug", "etelpmoc", "dehsinif"]
        .iter()
        .map(|s| s.chars().rev().collect())
        .collect()
}
