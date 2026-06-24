pub mod domain;
pub mod predicate;
pub mod presets;

pub use domain::*;
pub use predicate::*;
pub use presets::*;

use crate::Result;
use std::collections::HashMap;

/// The fully merged configuration state for a specific project.
#[derive(Debug, Default)]
pub struct LayeredConfig {
    pub targets: HashMap<String, SweepTarget>, // key: path
    pub noise_patterns: Vec<NoisePattern>,
}

impl LayeredConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge a base persona configuration into this layered config.
    pub fn merge_persona(&mut self, persona: PersonaConfig) {
        for target in persona.sweep_target {
            self.targets.insert(target.path.clone(), target);
        }
        for noise in persona.noise_pattern {
            self.noise_patterns.push(noise);
        }
    }

    /// Merge project-specific overrides.
    /// Overrides replace existing rules for the same path.
    pub fn merge_project_override(&mut self, project: ProjectConfig) {
        for target in project.override_target {
            self.targets.insert(target.path.clone(), target);
        }
    }
}

/// Helper function to load and parse a preset
pub fn load_preset(name: &str) -> Result<PersonaConfig> {
    let toml_str = get_preset_toml(name).unwrap_or(GLOBAL_DEFAULTS_TOML);
    toml::from_str(toml_str).map_err(crate::SprawlError::ConfigParse)
}
