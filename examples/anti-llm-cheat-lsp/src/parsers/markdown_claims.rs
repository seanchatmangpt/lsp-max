use crate::observations::Observation;

const VICTORY_PHRASES: &[(&str, &str)] = &[
    ("victory confirmed", "Victory confirmed"),
    ("fully admitted", "fully admitted"),
    ("all gaps resolved", "all gaps resolved"),
    ("successfully proven", "successfully proven"),
    ("path is clear", "path is clear"),
    ("routing to packplan", "Routing to PackPlan"),
    ("staging -> mutationgate", "Staging -> MutationGate"),
    ("test result: ok", "test result: ok"),
    ("log message treated as receipt", "LogMessage => Receipt"),
];

const PHRASE_KIND: &[&str] = &[
    "markdown_claim", // victory confirmed
    "markdown_claim", // fully admitted
    "markdown_claim", // all gaps resolved
    "markdown_claim", // successfully proven
    "markdown_claim", // path is clear
    "markdown_claim", // routing to packplan
    "markdown_claim", // staging -> mutationgate
    "markdown_claim", // test result: ok
    "markdown_claim", // log message treated as receipt
];

const PHRASE_MESSAGE: &[&str] = &[
    "Victory/overclaim language 'Victory confirmed' found in markdown",
    "Victory/overclaim language 'fully admitted' found in markdown",
    "Victory/overclaim language 'all gaps resolved' found in markdown",
    "Victory/overclaim language 'successfully proven' found in markdown",
    "Victory/overclaim language 'path is clear' found in markdown",
    "Unverified route claim 'Routing to PackPlan' found in markdown",
    "Unverified route claim 'Staging -> MutationGate' found in markdown",
    "Fake receipt claim 'test result: ok' found in markdown",
    "Fake receipt claim 'LogMessage => Receipt' found in markdown",
];

pub fn parse_markdown_claims(filepath: &str, content: &str) -> Vec<Observation> {
    let mut obs = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_lower = line.to_lowercase();

        for (i, &(phrase, orig)) in VICTORY_PHRASES.iter().enumerate() {
            // Single find — no double-scan
            if let Some(col0) = line_lower.find(phrase) {
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: col0,
                    end_byte: col0 + phrase.len(),
                    line: line_idx + 1,
                    column: col0 + 1,
                    kind: PHRASE_KIND[i].to_string(),
                    construct: orig.to_string(),
                    context: line.trim().to_string(),
                    message: PHRASE_MESSAGE[i].to_string(),
                });
            }
        }
    }

    obs
}
