use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CounterfactualProbe {
    RemoveFile { path: String },
    RemoveMarker { marker: String },
    CorruptJsonField { field: String },
    AddNthItem { n: usize },
    DisableChecker { checker_id: String },
    SwapAuthority { from: String, to: String },
    HideReceipt,
    HideOCEL,
    DoubleAcquireBuildSlot,
    DirectHeavyCommandWithoutSlot,
    Custom { id: String },
}

impl CounterfactualProbe {
    pub fn name(&self) -> String {
        match self {
            Self::RemoveFile { .. } => "RemoveFile".to_string(),
            Self::RemoveMarker { .. } => "RemoveMarker".to_string(),
            Self::CorruptJsonField { .. } => "CorruptJsonField".to_string(),
            Self::AddNthItem { .. } => "AddNthItem".to_string(),
            Self::DisableChecker { .. } => "DisableChecker".to_string(),
            Self::SwapAuthority { .. } => "SwapAuthority".to_string(),
            Self::HideReceipt => "HideReceipt".to_string(),
            Self::HideOCEL => "HideOCEL".to_string(),
            Self::DoubleAcquireBuildSlot => "DoubleAcquireBuildSlot".to_string(),
            Self::DirectHeavyCommandWithoutSlot => "DirectHeavyCommandWithoutSlot".to_string(),
            Self::Custom { id } => id.clone(),
        }
    }
}
