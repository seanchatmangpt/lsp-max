use crate::jsonrpc::Result;
use max_protocol::{IntentValidateParams, IntentValidateResult, IntentOutcome};
use wasm4pm_cognition::breeds::{BreedInput, CognitionBreed};
use wasm4pm_cognition::breeds::prolog::Prolog;

/// Validates an agent's intent using wasm4pm cognitive breeds.
/// This implements the doctrine that "MCP & A2A are downstream of LSP"
/// by evaluating natural language and intents via cognitive WASM engines.
pub async fn max_intent_validate(params: IntentValidateParams) -> Result<IntentValidateResult> {
    // 1. We instantiate a cognitive breed (e.g., Prolog).
    // In a full implementation, we'd route this based on the intent kind, 
    // but here we demonstrate the cognitive validation of language constraints.
    let breed = Prolog;
    
    // 2. We construct a Cognitive Breed Input from the agent's intent.
    let input = BreedInput {
        intent: format!("Validate intent {}", params.intent_id),
        ..Default::default()
    };
    
    // 3. We run the WASM Cognitive Breed (the native NLP/Reasoning engine of LSP).
    match breed.run(&input) {
        Ok(output) => {
            let is_valid = !output.facts.is_empty() || !output.explanation.is_empty();
            
            let outcome = if is_valid {
                IntentOutcome::Cleared
            } else {
                IntentOutcome::Blocked { 
                    reason: "Cognitive validation rejected the intent due to insufficient standing.".to_string() 
                }
            };
            
            Ok(IntentValidateResult {
                intent_id: params.intent_id,
                valid: is_valid,
                outcome,
                violations: vec![],
            })
        }
        Err(err) => {
            Ok(IntentValidateResult {
                intent_id: params.intent_id,
                valid: false,
                outcome: IntentOutcome::Blocked { reason: format!("Cognition failure: {}", err) },
                violations: vec![],
            })
        }
    }
}
