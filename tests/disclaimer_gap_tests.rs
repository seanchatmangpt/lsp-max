#![allow(dead_code)]
#![allow(clippy::collapsible_match)]

use anyhow::{anyhow, Result};
#[derive(Debug, PartialEq, Clone)]
enum AdmissionStatus {
    Candidate,
    ProjectTruth,
}

#[derive(Debug, PartialEq, Clone)]
enum Evidence {
    AgentSummary { text: String },
    TestOutput { stdout: String },
    ModelDisclaimer { text: String },
    AgentClaim { text: String },
    MechanicalWitness { receipt: BoundedReceipt },
}

#[derive(Debug, PartialEq, Clone)]
struct BoundedObservation {
    state: String,
}

#[derive(Debug, PartialEq, Clone)]
struct Action {
    mutation: String,
}

#[derive(Debug, PartialEq, Clone)]
struct BoundedReceipt {
    action_digest: String,
    observation_digest: String,
}

struct DisclaimerGapGate;

impl DisclaimerGapGate {
    /// Applies the Chatman Equation: R_B ⊢ A = μ(O*_B)
    fn evaluate_admission(
        action: &Action,
        observation: &BoundedObservation,
        evidence: &[Evidence],
    ) -> Result<AdmissionStatus> {
        let mut has_mechanical_witness = false;
        let mut valid_receipt = false;

        let valid_action_digest = format!("hash({})", action.mutation);
        let valid_obs_digest = format!("hash({})", observation.state);

        for e in evidence {
            match e {
                Evidence::MechanicalWitness { receipt } => {
                    has_mechanical_witness = true;
                    if receipt.action_digest == valid_action_digest
                        && receipt.observation_digest == valid_obs_digest
                    {
                        valid_receipt = true;
                    } else {
                        return Err(anyhow!(
                            "Chatman Equation Failed: Invalid digest in receipt"
                        ));
                    }
                }
                Evidence::AgentClaim { text } => {
                    if text.contains("LSIF admitted") && !has_mechanical_witness {
                        return Err(anyhow!("LSPMAX-DISCLAIMER-GAP-OPEN: severity = STOP, admission_allowed = false. Claim: {}", text));
                    }
                }
                _ => {}
            }
        }

        if !has_mechanical_witness {
            return Err(anyhow!("DISCLAIMER_GAP_CLOSED: Candidate cannot become ProjectTruth without a mechanical witness / receipt"));
        }

        if !valid_receipt {
            return Err(anyhow!("DISCLAIMER_GAP_CLOSED: Receipt invalid"));
        }

        Ok(AdmissionStatus::ProjectTruth)
    }
}

#[test]
fn model_output_without_receipt_is_candidate() -> Result<()> {
    let observation = BoundedObservation {
        state: "valid_state".into(),
    };
    let action = Action {
        mutation: "valid_mutation".into(),
    };

    let result = DisclaimerGapGate::evaluate_admission(&action, &observation, &[]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("DISCLAIMER_GAP_CLOSED"));
    Ok(())
}

#[test]
fn agent_summary_cannot_admit_lsif() -> Result<()> {
    let observation = BoundedObservation {
        state: "lsif_state".into(),
    };
    let action = Action {
        mutation: "lsif_mutation".into(),
    };

    let evidence = vec![Evidence::AgentSummary {
        text: "LSIF is good to go".into(),
    }];

    let result = DisclaimerGapGate::evaluate_admission(&action, &observation, &evidence);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("DISCLAIMER_GAP_CLOSED"));
    Ok(())
}

#[test]
fn test_output_is_not_receipt() -> Result<()> {
    let observation = BoundedObservation {
        state: "code_state".into(),
    };
    let action = Action {
        mutation: "code_mutation".into(),
    };

    let evidence = vec![Evidence::TestOutput {
        stdout: "test passed".into(),
    }];

    let result = DisclaimerGapGate::evaluate_admission(&action, &observation, &evidence);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("DISCLAIMER_GAP_CLOSED"));
    Ok(())
}

#[test]
fn candidate_as_authority_refused() -> Result<()> {
    let observation = BoundedObservation {
        state: "state".into(),
    };
    let action = Action {
        mutation: "mutation".into(),
    };

    let evidence = vec![Evidence::AgentClaim {
        text: "model output is authority".into(),
    }];

    let result = DisclaimerGapGate::evaluate_admission(&action, &observation, &evidence);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("DISCLAIMER_GAP_CLOSED"));
    Ok(())
}

#[test]
fn disclaimer_gap_open_when_no_mechanical_witness() -> Result<()> {
    let observation = BoundedObservation { state: "s".into() };
    let action = Action {
        mutation: "m".into(),
    };

    let result = DisclaimerGapGate::evaluate_admission(&action, &observation, &[]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("mechanical witness"));
    Ok(())
}

#[test]
fn model_disclaimer_not_treated_as_safety() -> Result<()> {
    let observation = BoundedObservation { state: "s".into() };
    let action = Action {
        mutation: "m".into(),
    };

    let evidence = vec![Evidence::ModelDisclaimer {
        text: "I might be wrong".into(),
    }];

    let result = DisclaimerGapGate::evaluate_admission(&action, &observation, &evidence);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("DISCLAIMER_GAP_CLOSED"));
    Ok(())
}

#[test]
fn agent_claims_admitted_without_receipt_refused() -> Result<()> {
    let observation = BoundedObservation {
        state: "lsif".into(),
    };
    let action = Action {
        mutation: "update".into(),
    };

    let evidence = vec![Evidence::AgentClaim {
        text: "LSIF admitted because the model says the tests passed.".into(),
    }];

    let result = DisclaimerGapGate::evaluate_admission(&action, &observation, &evidence);
    assert!(result.is_err());
    let err_str = result.as_ref().unwrap_err().to_string();
    assert!(err_str.contains("LSPMAX-DISCLAIMER-GAP-OPEN"));
    assert!(err_str.contains("severity = STOP"));
    assert!(err_str.contains("admission_allowed = false"));
    Ok(())
}

#[test]
fn test_disclaimer_gap_closed_invariant() -> Result<()> {
    let observation = BoundedObservation {
        state: "valid_state".into(),
    };
    let action = Action {
        mutation: "valid_mutation".into(),
    };

    let valid_receipt = BoundedReceipt {
        action_digest: "hash(valid_mutation)".into(),
        observation_digest: "hash(valid_state)".into(),
    };

    let evidence = vec![Evidence::MechanicalWitness {
        receipt: valid_receipt,
    }];
    let status = DisclaimerGapGate::evaluate_admission(&action, &observation, &evidence)?;

    assert_eq!(status, AdmissionStatus::ProjectTruth);
    Ok(())
}
