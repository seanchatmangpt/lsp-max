//! Attribute parsers (agent contract: `attributes`).
//!
//! - `#[allow_isel_residual]` — the function may keep an instruction-
//!   selection residual (branches emitted by rustc's ISel, e.g. from
//!   bounds checks). Classification: HINT-residual.
//! - `#[branchless_invariant]` — declares a hard branchless invariant.
//!   Court finds branches under this attribute → ERROR-refused.
//!
//! The parsed attributes + the Tier 2 court's verdict jointly decide
//! admission (see `receipts::ReceiptVector`).

/// Kind of a parsed branchless-lsp attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrKind {
    /// `#[allow_isel_residual]` — HINT-class residual allowance.
    AllowIselResidual,
    /// `#[branchless_invariant]` — declares a hard branchless invariant.
    BranchlessInvariant,
}

/// Why an attribute parse failed (typed; WG3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttrParseErrorKind {
    /// Path not a known branchless attribute.
    #[default]
    UnknownAttr,
    /// Parameterized form carried a key outside
    /// {target, max_branches, reason}.
    UnknownParamKey,
    /// Parameterized form had a malformed or wrong-typed value.
    MalformedParam,
    /// The same key appeared twice.
    DuplicateParam,
    /// `(` without a closing `)`.
    MissingCloseParen,
}

/// Parse failure naming the offending attribute path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttrParseError {
    /// Offending attribute path or parameter key.
    pub attr: String,
    /// Typed failure reason.
    pub kind: AttrParseErrorKind,
}

impl std::fmt::Display for AttrParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let why = match self.kind {
            AttrParseErrorKind::UnknownAttr => "unknown branchless attribute",
            AttrParseErrorKind::UnknownParamKey => "unknown parameter key",
            AttrParseErrorKind::MalformedParam => "malformed parameter value",
            AttrParseErrorKind::DuplicateParam => "duplicate parameter key",
            AttrParseErrorKind::MissingCloseParen => "missing closing paren",
        };
        write!(f, "{why}: {}", self.attr)
    }
}

impl std::error::Error for AttrParseError {}

/// Parameterized `#[allow_isel_residual(...)]` payload (WG3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResidualParams {
    /// Named target, e.g. `riscv32`. The allowance applies only on
    /// matching targets (`any` matches all; a prefix match also holds,
    /// so `riscv32` covers `riscv32imc`).
    pub target: String,
    /// Exact allowed machine-branch budget; `None` = unmeasured.
    pub max_branches: Option<usize>,
    /// Literal reason carried into diagnostics + receipt provenance.
    pub reason: Option<String>,
}

impl ResidualParams {
    /// Does this allowance govern `target`?
    pub fn target_matches(&self, target: &str) -> bool {
        self.target == "any"
            || target == self.target.as_str()
            || target.starts_with(self.target.as_str())
    }

    /// Classification of a measured branch count under this allowance.
    pub fn classify_count(&self, branches: usize) -> Classification {
        match branches {
            0 => Classification::Admitted,
            b if Some(b) <= self.max_branches => Classification::HintResidual,
            _ => Classification::ErrorRefused,
        }
    }

    /// Provenance line for diagnostics + receipts (WG3): the literal
    /// reason plus the budget. Git-blame provenance is future work (not
    /// available in-process today; see lane receipt).
    pub fn reason_note(&self) -> String {
        let max = self
            .max_branches
            .map(|m| m.to_string())
            .unwrap_or_else(|| "unmeasured".to_string());
        let reason = self.reason.clone().unwrap_or_default();
        format!(
            "allow_isel_residual(target = \"{}\", max_branches = {}, \
             reason = \"{}\")",
            self.target, max, reason
        )
    }
}

/// Attributes parsed from one function item.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedAttrs {
    /// Bare `#[allow_isel_residual]` seen (bare-form semantics only).
    pub allow_isel_residual: bool,
    /// Any `#[branchless_invariant]` seen.
    pub branchless_invariant: bool,
    /// Parameterized `#[allow_isel_residual(...)]` seen (WG3). Mutually
    /// exclusive with the bare bool by construction: the parameterized
    /// form does NOT set the bare flag, so mismatched-target fallback
    /// keeps bare (refusing) semantics.
    pub residual_params: Option<ResidualParams>,
}

/// Parse one parameterized payload: `key = value` comma pairs.
fn parse_residual_params(s: &str) -> Result<ResidualParams, AttrParseError> {
    let mut p = ResidualParams::default();
    let mut seen_target = false;
    let mut seen_max = false;
    let mut seen_reason = false;
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(AttrParseError {
                attr: s.to_string(),
                kind: AttrParseErrorKind::MalformedParam,
            });
        }
        let (key, value) = part.split_once('=').ok_or(AttrParseError {
            attr: part.to_string(),
            kind: AttrParseErrorKind::MalformedParam,
        })?;
        let key = key.trim();
        let value = value.trim();
        let unquote = |v: &str| -> Result<String, AttrParseError> {
            let inner = v
                .strip_prefix('"')
                .and_then(|r| r.strip_suffix('"'))
                .ok_or(AttrParseError {
                    attr: v.to_string(),
                    kind: AttrParseErrorKind::MalformedParam,
                })?;
            Ok(inner.to_string())
        };
        let dup = |seen: &mut bool, key: &str| -> Result<(), AttrParseError> {
            if *seen {
                return Err(AttrParseError {
                    attr: key.to_string(),
                    kind: AttrParseErrorKind::DuplicateParam,
                });
            }
            *seen = true;
            Ok(())
        };
        match key {
            "target" => {
                dup(&mut seen_target, key)?;
                p.target = unquote(value)?;
            }
            "max_branches" => {
                dup(&mut seen_max, key)?;
                p.max_branches = Some(value.parse::<usize>().map_err(|_| AttrParseError {
                    attr: value.to_string(),
                    kind: AttrParseErrorKind::MalformedParam,
                })?);
            }
            "reason" => {
                dup(&mut seen_reason, key)?;
                p.reason = Some(unquote(value)?);
            }
            k => {
                return Err(AttrParseError {
                    attr: k.to_string(),
                    kind: AttrParseErrorKind::UnknownParamKey,
                });
            }
        }
    }
    Ok(p)
}

/// Parse branchless-lsp attributes from raw attribute strings.
///
/// Accepts bare `#[attr]` and parameterized `#[allow_isel_residual(
/// target = "...", max_branches = N, reason = "...")]` forms; unknown
/// branchless-shaped paths (starting with `allow_isel` or `branchless`)
/// are refused with a typed error.
pub fn parse_attrs(attrs: &[String]) -> Result<ParsedAttrs, AttrParseError> {
    let mut out = ParsedAttrs::default();
    for raw in attrs {
        let path = raw
            .trim()
            .trim_start_matches('#')
            .trim_matches(|c| c == '[' || c == ']')
            .trim();
        let (name, params) = match path.split_once('(') {
            Some((n, rest)) => {
                let inner = rest
                    .strip_suffix(')')
                    .ok_or(AttrParseError {
                        attr: path.to_string(),
                        kind: AttrParseErrorKind::MissingCloseParen,
                    })?
                    .trim();
                (n.trim(), inner)
            }
            None => (path, ""),
        };
        match name {
            "allow_isel_residual" if params.is_empty() => out.allow_isel_residual = true,
            "allow_isel_residual" => out.residual_params = Some(parse_residual_params(params)?),
            "branchless_invariant" if params.is_empty() => out.branchless_invariant = true,
            "branchless_invariant" => {
                return Err(AttrParseError {
                    attr: path.to_string(),
                    kind: AttrParseErrorKind::UnknownParamKey,
                })
            }
            p if p.starts_with("allow_isel") || p.starts_with("branchless") => {
                return Err(AttrParseError {
                    attr: p.to_string(),
                    kind: AttrParseErrorKind::UnknownAttr,
                });
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Classify a court verdict against parsed attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// No branches found by the court.
    Admitted,
    /// Branches found, but `#[allow_isel_residual]` present.
    HintResidual,
    /// Branches found with `#[branchless_invariant]` — refusal.
    ErrorRefused,
    /// Branches found with no attribute — refusal.
    ErrorRefusedNoAttr,
}

/// Classify: attributes + did-the-court-find-branches → verdict class.
///
/// Bare-form behavior is unchanged (WG3). A parameterized allowance is
/// honored conservatively in the bool form: any branch with a budget of
/// at least 1 is HINT, budget 0 refuses. Exact-count courts use the
/// count forms (`classify_count`, `classify_seeded_count`,
/// `ResidualParams::classify_count`).
pub fn classify(attrs: &ParsedAttrs, branches_found: bool) -> Classification {
    if let Some(p) = &attrs.residual_params {
        if branches_found {
            return if p.max_branches == Some(0) {
                Classification::ErrorRefused
            } else {
                Classification::HintResidual
            };
        }
        return Classification::Admitted;
    }
    match (
        branches_found,
        attrs.allow_isel_residual,
        attrs.branchless_invariant,
    ) {
        (false, _, _) => Classification::Admitted,
        (true, true, _) => Classification::HintResidual,
        (true, false, true) => Classification::ErrorRefused,
        (true, false, false) => Classification::ErrorRefusedNoAttr,
    }
}

/// Embedded residual grid seed (F16/W16 item 5, lane LSPR).
///
/// Source of truth: cgap
/// `docs/jira/v26.11.2/countermeasures/residuals.toml` (sha256
/// cc62d41d9911e1f78be299327298322dcef62a0492aa7434e0fb42bf16d06409),
/// copied verbatim to `crates/branchless-lsp/residuals.toml` and
/// embedded at compile time so the seed ships inside the binary with
/// zero runtime path lookup.
pub const RESIDUALS_TOML: &str = include_str!("../residuals.toml");

/// Residual kind from the seed grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedKind {
    /// Documented branch(es) survive ISel (asserted exactly by census).
    IselResidual,
    /// 0-branch HOT cell (post-repair PASS).
    Residual,
    /// SKIP-REFERENCE: presence required, no 0-branch claim.
    SkipReference,
}

impl SeedKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "isel_residual" => Some(Self::IselResidual),
            "repaired_hot" => Some(Self::Residual),
            "skip_reference" => Some(Self::SkipReference),
            _ => None,
        }
    }
}

/// One seed row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualRow {
    /// Crate-local fn path, e.g. `select::select_slice`.
    pub symbol: String,
    /// `aarch64` | `riscv32imc` | `any`.
    pub target: String,
    /// Residual kind.
    pub kind: SeedKind,
    /// Exact allowed branch count (-1 = not censused).
    pub branch_count: i64,
    /// Receipt path:line cite.
    pub provenance: String,
}

/// Parsed residual grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualGrid {
    rows: Vec<ResidualRow>,
}

/// Verdict for a symbol under grid seeding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedVerdict {
    /// Grid row matched: auto-RESIDUAL/HINT with provenance, no
    /// `#[allow_isel_residual]` annotation needed.
    Seeded { row: ResidualRow },
    /// No grid row matched — refusal path unchanged.
    Unknown,
}

impl ResidualGrid {
    /// Parse the grid from TOML text.
    pub fn from_str(text: &str) -> Result<Self, String> {
        let raw: Vec<_> = toml::from_str::<toml::Value>(text)
            .map_err(|e| format!("residuals.toml parse: {e}"))?
            .get("residual")
            .and_then(|v| v.as_array())
            .ok_or("residuals.toml missing [[residual]] array")?
            .clone();
        let mut rows = Vec::new();
        for r in raw {
            let g = |k: &str| {
                r.get(k)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| format!("row missing {k}"))
            };
            rows.push(ResidualRow {
                symbol: g("symbol")?,
                target: g("target")?,
                kind: SeedKind::parse(&g("residual_kind")?)
                    .ok_or_else(|| "bad residual_kind".to_string())?,
                branch_count: r
                    .get("branch_count")
                    .and_then(|v| v.as_integer())
                    .ok_or_else(|| "row missing branch_count".to_string())?,
                provenance: g("provenance")?,
            });
        }
        Ok(Self { rows })
    }

    /// The canonical embedded grid.
    pub fn canonical() -> Self {
        Self::from_str(RESIDUALS_TOML).expect("embedded residuals.toml parses")
    }

    /// Row count.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True when empty (kept for exhaustiveness; canonical grid is not).
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Borrow the parsed rows (provenance extraction for receipts).
    pub fn rows_ref(&self) -> &[ResidualRow] {
        &self.rows
    }

    /// Lookup: exact symbol match, target match or `any`.
    pub fn lookup(&self, symbol: &str, target: &str) -> SeedVerdict {
        // Bare fn names (e.g. `select_slice` from the disasm court) match
        // the crate-local path's last segment.
        let matches = |row_symbol: &str| {
            row_symbol == symbol || row_symbol.rsplit("::").next() == Some(symbol)
        };
        match self
            .rows
            .iter()
            .find(|r| matches(&r.symbol) && (r.target == target || r.target == "any"))
        {
            Some(row) => SeedVerdict::Seeded { row: row.clone() },
            None => SeedVerdict::Unknown,
        }
    }

    /// Classify a function with grid seeding: a seeded symbol is
    /// RESIDUAL/HINT WITHOUT any attribute; unknown symbols fall back to
    /// the attribute-only path (REFUSED when branchy and unannotated).
    /// The bool form delegates to the count form with
    /// `branches_found as usize` (WG3): bool callers keep identical
    /// behavior for bare attrs; exact-count courts use
    /// `classify_seeded_count`.
    pub fn classify_seeded(
        &self,
        symbol: &str,
        target: &str,
        attrs: &ParsedAttrs,
        branches_found: bool,
    ) -> Classification {
        self.classify_seeded_count(symbol, target, attrs, usize::from(branches_found))
    }

    /// Count-exact seeded classification (WG3). A parameterized
    /// `#[allow_isel_residual(target, max_branches, reason)]` supersedes
    /// the bare count semantics for that symbol: branches ≤ max on the
    /// named target → RESIDUAL/HINT; > max → REFUSED. Bare-form behavior
    /// is unchanged.
    pub fn classify_seeded_count(
        &self,
        symbol: &str,
        target: &str,
        attrs: &ParsedAttrs,
        branches: usize,
    ) -> Classification {
        // Parameterized allowance governs this symbol when present.
        if let Some(p) = &attrs.residual_params {
            if p.target_matches(target) {
                return p.classify_count(branches);
            }
        }
        if let SeedVerdict::Seeded { .. } = self.lookup(symbol, target) {
            if branches > 0 {
                return Classification::HintResidual;
            }
            return Classification::Admitted;
        }
        classify_count(attrs, branches)
    }
}

/// Bare-form classification over an exact branch count (WG3).
pub fn classify_count(attrs: &ParsedAttrs, branches: usize) -> Classification {
    if branches == 0 {
        return Classification::Admitted;
    }
    if attrs.allow_isel_residual {
        return Classification::HintResidual;
    }
    if attrs.branchless_invariant {
        return Classification::ErrorRefused;
    }
    Classification::ErrorRefusedNoAttr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_attributes() {
        let a = parse_attrs(&[
            "#[allow_isel_residual]".into(),
            "branchless_invariant".into(),
        ])
        .expect("parse ok");
        assert!(a.allow_isel_residual && a.branchless_invariant);
    }

    #[test]
    fn unknown_branchless_attr_is_refused() {
        let e = parse_attrs(&["branchless_frobnicate".into()]).unwrap_err();
        assert_eq!(e.attr, "branchless_frobnicate");
    }

    #[test]
    fn classify_matrix() {
        assert_eq!(
            classify(&ParsedAttrs::default(), false),
            Classification::Admitted
        );
        let hint = ParsedAttrs {
            allow_isel_residual: true,
            branchless_invariant: false,
            ..Default::default()
        };
        assert_eq!(classify(&hint, true), Classification::HintResidual);
        let inv = ParsedAttrs {
            allow_isel_residual: false,
            branchless_invariant: true,
            ..Default::default()
        };
        assert_eq!(classify(&inv, true), Classification::ErrorRefused);
        assert_eq!(
            classify(&ParsedAttrs::default(), true),
            Classification::ErrorRefusedNoAttr
        );
    }

    #[test]
    fn classification_non_vacuous_matrix() {
        // Anti-vacuity: flipping branches_found flips every non-Admitted cell.
        let hint = ParsedAttrs {
            allow_isel_residual: true,
            branchless_invariant: false,
            ..Default::default()
        };
        assert_ne!(classify(&hint, true), classify(&hint, false));
    }

    // ---- LSPR seed courts (anti-vacuity: tampered grid must flip) ----

    #[test]
    fn canonical_grid_parses_with_exact_row_count() {
        let grid = ResidualGrid::canonical();
        assert_eq!(grid.len(), 10, "canonical grid row count pinned");
        let r = grid.lookup("select_slice", "aarch64");
        let SeedVerdict::Seeded { row } = r else {
            panic!("select_slice/aarch64 must seed");
        };
        assert_eq!(row.branch_count, 1, "cbz empty-mask default, exact");
        assert_eq!(row.kind, SeedKind::IselResidual);
        assert!(row.provenance.contains("repair-01-branchless-census.md"));
    }

    #[test]
    fn seeded_symbol_is_residual_without_annotation() {
        let grid = ResidualGrid::canonical();
        for (sym, tgt) in [
            ("select_slice", "aarch64"),
            ("select_slice", "riscv32imc"),
            ("cmov_u64", "aarch64"),
            ("cmov_u64", "riscv32imc"),
            ("eytzinger_restore", "aarch64"),
            ("eytzinger_restore", "riscv32imc"),
            ("saturated_subs_p28", "aarch64"), // target=any row
        ] {
            let cls = grid.classify_seeded(sym, tgt, &ParsedAttrs::default(), true);
            assert_eq!(
                cls,
                Classification::HintResidual,
                "{sym}/{tgt} must auto-RESIDUAL without annotation"
            );
        }
        assert_eq!(
            grid.classify_seeded("select_slice", "aarch64", &ParsedAttrs::default(), false,),
            Classification::Admitted
        );
    }

    #[test]
    fn unknown_symbol_still_refused_even_when_seeded_grid_present() {
        let grid = ResidualGrid::canonical();
        assert_eq!(
            grid.lookup("totally_unknown_fn", "aarch64"),
            SeedVerdict::Unknown
        );
        assert_eq!(
            grid.classify_seeded(
                "totally_unknown_fn",
                "aarch64",
                &ParsedAttrs::default(),
                true,
            ),
            Classification::ErrorRefusedNoAttr
        );
    }

    #[test]
    fn skip_reference_rows_seed_with_negative_count() {
        let grid = ResidualGrid::canonical();
        let n: usize = grid
            .rows_ref()
            .iter()
            .filter(|r| r.kind == SeedKind::SkipReference)
            .count();
        assert_eq!(n, 3, "3 SKIP-REFERENCE rows pinned");
    }

    #[test]
    fn tampered_branch_count_is_detected() {
        // Tamper a /tmp sandbox copy: inflate select_slice/aarch64's
        // branch_count 1 -> 9. The court must DETECT (grid content
        // diverges from the pinned canonical cell), never admit.
        let tampered = RESIDUALS_TOML.replace(
            "branch_count = 1  # cbz: empty-mask default",
            "branch_count = 9  # cbz: empty-mask default",
        );
        assert_ne!(tampered, RESIDUALS_TOML, "tamper must take effect");
        let grid = ResidualGrid::from_str(&tampered).expect("parse");
        let SeedVerdict::Seeded { row } = grid.lookup("select_slice", "aarch64") else {
            panic!("row still present");
        };
        assert_ne!(
            row.branch_count, 1,
            "tampered branch_count must diverge from pinned 1"
        );
        let canonical = ResidualGrid::canonical();
        let SeedVerdict::Seeded { row: c } = canonical.lookup("select_slice", "aarch64") else {
            panic!("canonical must seed");
        };
        assert_eq!(
            c.branch_count, 1,
            "canonical cell remains exactly 1; tampered grid DETECTED"
        );
        assert_ne!(
            grid, canonical,
            "tampered grid must not equal canonical grid"
        );
    }

    #[test]
    fn target_mismatch_does_not_seed() {
        let grid = ResidualGrid::canonical();
        assert_eq!(
            grid.lookup("select_slice", "x86_64"),
            SeedVerdict::Unknown,
            "no x86_64 rows: unseeded target stays Unknown"
        );
    }

    // ---- WG3: parameterized #[allow_isel_residual(...)] courts ----

    fn param_attrs(max: Option<usize>) -> ParsedAttrs {
        let mut src =
            format!("#[allow_isel_residual(target = \"riscv32\", reason = \"measured\")]");
        if let Some(m) = max {
            src = format!(
                "#[allow_isel_residual(target = \"riscv32\", max_branches = {m}, \
                 reason = \"measured\")]"
            );
        }
        parse_attrs(&[src]).expect("parameterized attr parses")
    }

    #[test]
    fn parameterized_attr_parses_full_payload() {
        let a = parse_attrs(&[concat!(
            "#[allow_isel_residual(target = \"riscv32\", ",
            "max_branches = 3, reason = \"measured\")]"
        )
        .into()])
        .expect("parse ok");
        assert!(a.allow_isel_residual == false, "bare flag must stay unset");
        let p = a.residual_params.expect("params present");
        assert_eq!(p.target, "riscv32");
        assert_eq!(p.max_branches, Some(3));
        assert_eq!(p.reason.as_deref(), Some("measured"));
    }

    #[test]
    fn unmeasured_parameterized_form_has_no_budget() {
        let a = parse_attrs(&[
            "#[allow_isel_residual(target = \"riscv32\", reason = \"unmeasured\")]".into(),
        ])
        .expect("parse ok");
        let p = a.residual_params.expect("params present");
        assert_eq!(p.max_branches, None);
    }

    #[test]
    fn unknown_param_key_is_typed_refused() {
        let e = parse_attrs(&["#[allow_isel_residual(wibble = 1)]".into()]).unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::UnknownParamKey);
        assert_eq!(e.attr, "wibble");
    }

    #[test]
    fn malformed_and_duplicate_params_are_typed_refused() {
        let e =
            parse_attrs(&["#[allow_isel_residual(max_branches = \"three\")]".into()]).unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::MalformedParam);
        let e = parse_attrs(&["#[allow_isel_residual(target = \"a\", target = \"b\")]".into()])
            .unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::DuplicateParam);
        let e = parse_attrs(&["#[allow_isel_residual(target = \"a\"".into()]).unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::MissingCloseParen);
    }

    #[test]
    fn parameterized_branchless_invariant_is_typed_refused() {
        let e = parse_attrs(&["#[branchless_invariant(x = 1)]".into()]).unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::UnknownParamKey);
    }

    #[test]
    fn residual_at_exactly_max_branches_refused_at_max_plus_one() {
        let grid = ResidualGrid::canonical();
        let attrs = param_attrs(Some(2));
        assert_eq!(
            grid.classify_seeded_count("totally_unknown_fn", "riscv32imc", &attrs, 2),
            Classification::HintResidual,
            "branches == max_branches is RESIDUAL/HINT"
        );
        assert_eq!(
            grid.classify_seeded_count("totally_unknown_fn", "riscv32imc", &attrs, 3),
            Classification::ErrorRefused,
            "branches == max_branches + 1 is REFUSED"
        );
        assert_eq!(
            grid.classify_seeded_count("totally_unknown_fn", "riscv32imc", &attrs, 0),
            Classification::Admitted
        );
    }

    #[test]
    fn parameterized_allowance_is_target_scoped() {
        let grid = ResidualGrid::canonical();
        let attrs = param_attrs(Some(1));
        // riscv32 covers riscv32imc (prefix); within budget.
        assert_eq!(
            grid.classify_seeded_count("unknown_fn", "riscv32imc", &attrs, 1),
            Classification::HintResidual
        );
        assert_eq!(
            grid.classify_seeded_count("unknown_fn", "riscv32imc", &attrs, 2),
            Classification::ErrorRefused,
            "prefix target scoping does not relax the budget"
        );
        // On aarch64 the allowance does not govern: unseeded + branchy
        // + no bare attr → no-attr refusal (bare semantics preserved).
        assert_eq!(
            grid.classify_seeded_count("unknown_fn", "aarch64", &attrs, 1),
            Classification::ErrorRefusedNoAttr
        );
    }

    #[test]
    fn reason_surfaces_in_provenance_note() {
        let attrs = param_attrs(Some(2));
        let p = attrs.residual_params.expect("params");
        let note = p.reason_note();
        assert!(
            note.contains("reason = \"measured\""),
            "literal reason must surface: {note}"
        );
        assert!(note.contains("max_branches = 2"));
        assert!(note.contains("target = \"riscv32\""));
        // Unmeasured form reports its gap explicitly, never silently.
        let un = parse_attrs(&[
            "#[allow_isel_residual(target = \"riscv32\", reason = \"unmeasured\")]".into(),
        ])
        .unwrap();
        assert!(un
            .residual_params
            .unwrap()
            .reason_note()
            .contains("max_branches = unmeasured"));
    }

    #[test]
    fn bare_form_behavior_unchanged_under_parameterized_extension() {
        // Bare attr: budget-free HINT regardless of count (old semantics).
        let grid = ResidualGrid::canonical();
        let bare = parse_attrs(&["#[allow_isel_residual]".into()]).unwrap();
        assert!(bare.residual_params.is_none() && bare.allow_isel_residual);
        assert_eq!(
            grid.classify_seeded_count("unknown_fn", "aarch64", &bare, 99),
            Classification::HintResidual
        );
        assert_eq!(classify(&bare, true), Classification::HintResidual);
        // Bool form with parameterized attrs: conservative budget gate.
        assert_eq!(
            classify(&param_attrs(Some(0)), true),
            Classification::ErrorRefused
        );
        assert_eq!(
            classify(&param_attrs(Some(4)), true),
            Classification::HintResidual
        );
        // Invariant refusal path untouched.
        let inv = parse_attrs(&["branchless_invariant".into()]).unwrap();
        assert_eq!(classify(&inv, true), Classification::ErrorRefused);
    }

    // ---- G04: round-trip + positional-form fail-closed pinning ----

    #[test]
    fn key_value_form_round_trips_through_reason_note() {
        // Round-trip: literal attribute text → parsed params → the
        // provenance note reproduces every literal field.
        let a = parse_attrs(&[concat!(
            "#[allow_isel_residual(target = \"riscv32imc\", ",
            "max_branches = 4, reason = \"cbz empty-mask\")]"
        )
        .into()])
        .expect("key-value form parses");
        let p = a.residual_params.as_ref().expect("params present");
        assert_eq!(p.target, "riscv32imc");
        assert_eq!(p.max_branches, Some(4));
        assert_eq!(p.reason.as_deref(), Some("cbz empty-mask"));
        let note = p.reason_note();
        assert!(note.contains("target = \"riscv32imc\""));
        assert!(note.contains("max_branches = 4"));
        assert!(note.contains("reason = \"cbz empty-mask\""));
        // The round-tripped note re-parses under the same grammar.
        let re = parse_attrs(&[format!("#[allow_isel_residual({})]", {
            let n = p.reason_note();
            n.trim_start_matches("allow_isel_residual(")
                .trim_end_matches(')')
                .to_string()
        })])
        .expect("round-tripped note re-parses");
        assert_eq!(
            a.residual_params, re.residual_params,
            "round-trip is stable: parse(note) == original"
        );
    }

    #[test]
    fn positional_form_is_fail_closed_typed_refused() {
        // There is deliberately NO positional form (bare or key=value
        // only). A positional payload must fail closed with a typed
        // error, never panic and never silently parse.
        let e = parse_attrs(&["#[allow_isel_residual(\"riscv32\", 3)]".into()]).unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::MalformedParam);
        let e = parse_attrs(&["#[allow_isel_residual(riscv32, 3)]".into()]).unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::MalformedParam);
    }

    #[test]
    fn mutant_non_numeric_max_branches_flips_happy_path() {
        // Anti-vacuity mutant: change the happy-path literal 3 → "3x".
        // The parse MUST refuse (MutantDetected); the earlier unquoted
        // 3 form MUST still parse (no vacuous refusal of the good form).
        let ok =
            parse_attrs(&["#[allow_isel_residual(target = \"riscv32\", max_branches = 3)]".into()])
                .expect("numeric happy path parses");
        assert_eq!(ok.residual_params.unwrap().max_branches, Some(3));
        let e = parse_attrs(&[
            "#[allow_isel_residual(target = \"riscv32\", max_branches = 3x)]".into(),
        ])
        .unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::MalformedParam);
        assert_eq!(e.attr, "3x");
    }

    #[test]
    fn mutant_duplicate_key_flips_happy_path() {
        // Anti-vacuity mutant: duplicating the key in an otherwise valid
        // payload must flip the parse from Ok to typed DuplicateParam.
        let e = parse_attrs(&[concat!(
            "#[allow_isel_residual(target = \"riscv32\", max_branches = 3, ",
            "target = \"aarch64\")]"
        )
        .into()])
        .unwrap_err();
        assert_eq!(e.kind, AttrParseErrorKind::DuplicateParam);
        assert_eq!(e.attr, "target");
    }

    #[test]
    fn classify_count_flips_across_budget_boundary() {
        // Anti-vacuity: the classification must flip at max, not before.
        for max in [1usize, 2, 5] {
            let attrs = param_attrs(Some(max));
            let p = attrs.residual_params.as_ref().unwrap();
            assert_eq!(p.classify_count(max), Classification::HintResidual);
            assert_eq!(p.classify_count(max + 1), Classification::ErrorRefused);
            assert_eq!(p.classify_count(0), Classification::Admitted);
        }
    }
}
