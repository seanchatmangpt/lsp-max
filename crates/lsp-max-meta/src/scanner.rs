/// A single law-axis violation found in a `.ttl` ontology file.
#[derive(Debug, Clone)]
pub struct Violation {
    /// 0-indexed line number.
    pub line: u32,
    pub col_start: u32,
    pub col_end: u32,
    pub message: String,
    pub severity: ViolationSeverity,
    pub code: String,
}

#[derive(Debug, Clone)]
pub enum ViolationSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Scan TTL content for law-axis violations and return all findings.
pub fn scan(content: &str) -> Vec<Violation> {
    let mut violations = vec![];
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let lnum = i as u32;

        // ANTI-LLM-META-001: forbidden plain tower-lsp / tower_lsp references
        for kw in &["tower-lsp", "tower_lsp"] {
            if let Some(col) = line.find(kw) {
                violations.push(Violation {
                    line: lnum,
                    col_start: col as u32,
                    col_end: (col + kw.len()) as u32,
                    message: format!(
                        "LawViolation: forbidden plain {kw} reference — use lsp-max"
                    ),
                    severity: ViolationSeverity::Error,
                    code: "ANTI-LLM-META-001".into(),
                });
            }
        }

        // ANTI-LLM-META-002: victory language in quoted string literals
        for kw in &[
            "\"done\"",
            "\"complete\"",
            "\"solved\"",
            "\"guaranteed\"",
            "\"all clean\"",
            "\"fully admitted\"",
        ] {
            if line.to_lowercase().contains(kw) {
                violations.push(Violation {
                    line: lnum,
                    col_start: 0,
                    col_end: line.len() as u32,
                    message: format!(
                        "VictoryLanguage: {kw} is forbidden — use bounded status: ADMITTED, CANDIDATE, OPEN, PARTIAL"
                    ),
                    severity: ViolationSeverity::Warning,
                    code: "ANTI-LLM-META-002".into(),
                });
            }
        }

        // ANTI-LLM-META-003: law:ADMITTED claimed without a law:receipt in the
        // surrounding context (next 5 lines).  Receipt must appear in scope for
        // the admission claim to be bounded.
        if line.contains("law:ADMITTED") || line.contains("law:status law:ADMITTED") {
            let has_receipt = lines[i..].iter().take(6).any(|l| l.contains("law:receipt"));
            if !has_receipt {
                violations.push(Violation {
                    line: lnum,
                    col_start: 0,
                    col_end: line.len() as u32,
                    message: "BLOCKED: law:ADMITTED claimed without law:receipt in scope — receipt chain OPEN".into(),
                    severity: ViolationSeverity::Warning,
                    code: "ANTI-LLM-META-003".into(),
                });
            }
        }

        // ANTI-LLM-META-004: lsp:Request declared with no law:status in the
        // next 7 lines — status defaults to UNKNOWN and must not be silently
        // collapsed.
        if line.contains("a lsp:Request")
            && !lines[i..].iter().take(8).any(|l| l.contains("law:status"))
        {
            violations.push(Violation {
                line: lnum,
                col_start: 0,
                col_end: line.len() as u32,
                message: "UNKNOWN: lsp:Request declared without law:status — status defaults to UNKNOWN; UNKNOWN must not collapse to ADMITTED or REFUSED".into(),
                severity: ViolationSeverity::Information,
                code: "ANTI-LLM-META-004".into(),
            });
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tower_lsp_reference() {
        let content = "# tower-lsp was here\n";
        let v = scan(content);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].code, "ANTI-LLM-META-001");
    }

    #[test]
    fn detects_victory_language() {
        let content = "law:description \"done\" .\n";
        let v = scan(content);
        assert!(v.iter().any(|x| x.code == "ANTI-LLM-META-002"));
    }

    #[test]
    fn detects_admitted_without_receipt() {
        let content = "law:status law:ADMITTED .\n";
        let v = scan(content);
        assert!(v.iter().any(|x| x.code == "ANTI-LLM-META-003"));
    }

    #[test]
    fn admitted_with_receipt_is_clear() {
        let content = "law:status law:ADMITTED .\nlaw:receipt \"sha256:abc\" .\n";
        let v = scan(content);
        assert!(!v.iter().any(|x| x.code == "ANTI-LLM-META-003"));
    }

    #[test]
    fn detects_request_without_status() {
        let content = "ex:Foo a lsp:Request .\n";
        let v = scan(content);
        assert!(v.iter().any(|x| x.code == "ANTI-LLM-META-004"));
    }
}
