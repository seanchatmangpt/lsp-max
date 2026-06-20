use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ===== Domain Tier =====
#[derive(Serialize, Clone)]
pub struct RuleInfo {
    pub code: String,
    pub category: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct RulesListResult {
    pub rules: Vec<RuleInfo>,
    pub total_count: usize,
}

#[derive(Serialize)]
pub struct RuleDescribeResult {
    pub rule: Option<RuleInfo>,
    pub found: bool,
}

// ===== Service Tier =====
pub struct RulesService;

impl RulesService {
    fn get_all_rules() -> Vec<RuleInfo> {
        vec![
            // Surface rules
            RuleInfo {
                code: "ANTI-LLM-SURFACE-001".to_string(),
                category: "surface".to_string(),
                description: "Plain tower-lsp reference detected".to_string(),
            },
            RuleInfo {
                code: "ANTI-LLM-SURFACE-003".to_string(),
                category: "surface".to_string(),
                description: "Observer dependency in observer pattern".to_string(),
            },
            RuleInfo {
                code: "ANTI-LLM-SURFACE-005".to_string(),
                category: "surface".to_string(),
                description: "Missing LSP 3.18 capability negotiation".to_string(),
            },
            // Authority rules
            RuleInfo {
                code: "ANTI-LLM-AUTH-002".to_string(),
                category: "authority".to_string(),
                description: "Fake CLAP abstraction detected".to_string(),
            },
            RuleInfo {
                code: "ANTI-LLM-AUTH-004".to_string(),
                category: "authority".to_string(),
                description: "String-shaped command authority".to_string(),
            },
            // Receipt rules
            RuleInfo {
                code: "ANTI-LLM-RECEIPT-001".to_string(),
                category: "receipts".to_string(),
                description: "Test stdout claimed as receipt".to_string(),
            },
            RuleInfo {
                code: "ANTI-LLM-RECEIPT-002".to_string(),
                category: "receipts".to_string(),
                description: "Log message used as receipt".to_string(),
            },
            RuleInfo {
                code: "ANTI-LLM-RECEIPT-003".to_string(),
                category: "receipts".to_string(),
                description: "Missing cryptographic digest".to_string(),
            },
            // Route rules
            RuleInfo {
                code: "ANTI-LLM-ROUTE-001".to_string(),
                category: "routes".to_string(),
                description: "Log output confused with route proof".to_string(),
            },
            RuleInfo {
                code: "ANTI-LLM-ROUTE-008".to_string(),
                category: "routes".to_string(),
                description: "Static analysis claimed as route proof".to_string(),
            },
            // Claims/victory language
            RuleInfo {
                code: "ANTI-LLM-CLAIM-004".to_string(),
                category: "claims".to_string(),
                description: "Victory language detected (done, solved, guaranteed)".to_string(),
            },
        ]
    }

    pub fn list_rules(category: Option<&str>) -> RulesListResult {
        let all = Self::get_all_rules();
        let filtered: Vec<_> = if let Some(cat) = category {
            all.into_iter().filter(|r| r.category == cat).collect()
        } else {
            all
        };

        let total_count = filtered.len();
        RulesListResult {
            rules: filtered,
            total_count,
        }
    }

    pub fn describe_rule(code: &str) -> RuleDescribeResult {
        let all = Self::get_all_rules();
        let rule = all.into_iter().find(|r| r.code == code);
        let found = rule.is_some();
        RuleDescribeResult { rule, found }
    }
}

// ===== Verb Tier (CLI) =====

#[verb("list")]
pub fn list_rules(category: Option<String>) -> Result<RulesListResult> {
    Ok(RulesService::list_rules(category.as_deref()))
}

#[verb("describe")]
pub fn describe_rule(code: String) -> Result<RuleDescribeResult> {
    Ok(RulesService::describe_rule(&code))
}
