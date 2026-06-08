use crate::observations::Observation;

pub fn parse_markdown_claims(filepath: &str, content: &str) -> Vec<Observation> {
    let mut obs = Vec::new();

    let victory_phrases = [
        ("victory confirmed", "Victory confirmed"),
        ("fully admitted", "fully admitted"),
        ("all gaps resolved", "all gaps resolved"),
        ("successfully proven", "successfully proven"),
        ("path is clear", "path is clear"),
    ];

    let route_claims = [
        ("routing to packplan", "Routing to PackPlan"),
        ("staging -> mutationgate", "Staging -> MutationGate"),
    ];

    let receipt_claims = [
        ("test result: ok", "test result: ok"),
        ("log message treated as receipt", "LogMessage => Receipt"),
    ];

    for (line_idx, line) in content.lines().enumerate() {
        let line_lower = line.to_lowercase();

        for &(phrase, orig) in &victory_phrases {
            if line_lower.contains(phrase) {
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: 0,
                    end_byte: 0,
                    line: line_idx + 1,
                    column: line_lower.find(phrase).unwrap_or(0) + 1,
                    kind: "markdown_claim".to_string(),
                    construct: orig.to_string(),
                    context: line.trim().to_string(),
                    message: format!("Victory/overclaim language '{}' found in markdown", orig),
                });
            }
        }

        for &(phrase, orig) in &route_claims {
            if line_lower.contains(phrase) {
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: 0,
                    end_byte: 0,
                    line: line_idx + 1,
                    column: line_lower.find(phrase).unwrap_or(0) + 1,
                    kind: "markdown_claim".to_string(),
                    construct: orig.to_string(),
                    context: line.trim().to_string(),
                    message: format!("Unverified route claim '{}' found in markdown", orig),
                });
            }
        }

        for &(phrase, orig) in &receipt_claims {
            if line_lower.contains(phrase) {
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: 0,
                    end_byte: 0,
                    line: line_idx + 1,
                    column: line_lower.find(phrase).unwrap_or(0) + 1,
                    kind: "markdown_claim".to_string(),
                    construct: orig.to_string(),
                    context: line.trim().to_string(),
                    message: format!("Fake receipt claim '{}' found in markdown", orig),
                });
            }
        }
    }

    obs
}
