use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    NukeSafe,
    NukeForce,
    Archive,
    Snooze,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepTarget {
    pub path: String,
    pub condition: String, // e.g. "idle_days > 14"
    pub action: Action,
    #[serde(default)]
    pub override_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoisePattern {
    pub pattern: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonaConfig {
    #[serde(default)]
    pub sweep_target: Vec<SweepTarget>,
    #[serde(default)]
    pub noise_pattern: Vec<NoisePattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub override_target: Vec<SweepTarget>,
}
