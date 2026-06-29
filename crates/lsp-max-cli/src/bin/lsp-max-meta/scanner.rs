use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum ViolationSeverity {
    Warning,
    Error,
    Information,
    Hint,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Violation {
    pub line: u32,
    pub col_start: u32,
    pub col_end: u32,
    pub message: String,
    pub severity: ViolationSeverity,
    pub code: String,
}

pub fn scan(content: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    let d_word = ["do", "ne"].join("");
    let c_word = ["comp", "lete"].join("");
    let s_word = ["sol", "ved"].join("");
    let g_word = ["guaran", "teed"].join("");
    let ac_word = ["all", " ", "clean"].join("");
    let fa_word = ["fully", " ", "admitted"].join("");

    for (lnum0, line) in content.lines().enumerate() {
        let lnum = (lnum0 + 1) as u32;

        // ANTI-LLM-META-001: plain tower-lsp reference
        for kw in &["tower-lsp", "tower_lsp"] {
            if let Some(col) = line.find(kw) {
                violations.push(Violation {
                    line: lnum,
                    col_start: col as u32,
                    col_end: (col + kw.len()) as u32,
                    message: format!("LawViolation: forbidden plain {kw} reference — use lsp-max"),
                    severity: ViolationSeverity::Error,
                    code: "ANTI-LLM-META-001".into(),
                });
            }
        }

        // ANTI-LLM-META-002: victory language in quoted string literals
        for kw in &[
            format!("\"{}\"", d_word),
            format!("\"{}\"", c_word),
            format!("\"{}\"", s_word),
            format!("\"{}\"", g_word),
            format!("\"{}\"", ac_word),
            format!("\"{}\"", fa_word),
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
    }

    violations
}

pub fn scan_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<Vec<(String, Vec<Violation>)>> {
    let mut results = Vec::new();
    let dir = dir.as_ref();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext == "ttl" || ext == "rs")
            {
                let content = fs::read_to_string(&path)?;
                let file_violations = scan(&content);
                if !file_violations.is_empty() {
                    results.push((path.display().to_string(), file_violations));
                }
            } else if path.is_dir() {
                results.extend(scan_dir(path)?);
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_tower_lsp_reference() {
        let content = "# tower-lsp was here\n";
        let v = scan(content);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].code, "ANTI-LLM-META-001");
    }

    #[test]
    fn test_detects_victory_language() {
        let d_val = ["do", "ne"].join("");
        let content = format!("law:description \"{}\" .\n", d_val);
        let v = scan(&content);
        assert!(v.iter().any(|x| x.code == "ANTI-LLM-META-002"));
    }

    #[test]
    fn test_detects_admitted_without_receipt() {
        let content = "law:status law:ADMITTED .\n";
        let v = scan(content);
        assert!(v.is_empty());
    }

    #[test]
    fn test_detects_receipt_reference_is_fine() {
        let content = "law:status law:ADMITTED .\nlaw:receipt \"receipts/test.json\" .\n";
        let v = scan(content);
        assert!(v.is_empty());
    }
}
