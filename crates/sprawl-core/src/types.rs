use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackInfo {
    pub primary_ecosystem: String,
    pub entry_points: Vec<String>,
    pub package_manager: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentTier {
    Safe,
    Dormant,
    NeedsReview,
    Destructive,
}
