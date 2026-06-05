use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// PolicyState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyState {
    Operational,
    ClarificationRequested,
    RefundAuthorized,
    /// Clean slate — used after max/reset to mark an instance ready for reuse.
    Active,
}

impl std::str::FromStr for PolicyState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Operational" => Ok(Self::Operational),
            "ClarificationRequested" => Ok(Self::ClarificationRequested),
            "RefundAuthorized" => Ok(Self::RefundAuthorized),
            "Active" => Ok(Self::Active),
            other => Err(format!("Unknown policy state: {other}")),
        }
    }
}

impl std::fmt::Display for PolicyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyState::Operational => write!(f, "Operational"),
            PolicyState::ClarificationRequested => write!(f, "ClarificationRequested"),
            PolicyState::RefundAuthorized => write!(f, "RefundAuthorized"),
            PolicyState::Active => write!(f, "Active"),
        }
    }
}

#[cfg(test)]
mod policy_state_tests {
    use super::PolicyState;
    #[test]
    fn test_policy_state_from_str_roundtrip() {
        assert_eq!(
            "Operational".parse::<PolicyState>(),
            Ok(PolicyState::Operational)
        );
        assert_eq!(
            "ClarificationRequested".parse::<PolicyState>(),
            Ok(PolicyState::ClarificationRequested)
        );
        assert_eq!(
            "RefundAuthorized".parse::<PolicyState>(),
            Ok(PolicyState::RefundAuthorized)
        );
        assert!("Bogus".parse::<PolicyState>().is_err());
    }
}
