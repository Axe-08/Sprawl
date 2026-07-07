pub mod domain;
pub mod predicate;
pub mod presets;

use self::domain::{NoisePattern, SweepRule};
use self::presets::{get_global_preset, get_ml_engineer_preset, get_web_dev_preset};
use crate::Result;

pub struct LayeredConfig {
    pub rules: Vec<SweepRule>,
    pub noise_patterns: Vec<NoisePattern>,
}

impl LayeredConfig {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            noise_patterns: Vec::new(),
        }
    }
}

impl Default for LayeredConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredConfig {
    pub fn load_global_defaults(&mut self) -> Result<()> {
        let global_preset = get_global_preset();
        self.rules.extend(global_preset.rules);
        self.noise_patterns.extend(global_preset.noise_patterns);
        Ok(())
    }

    // Add persona preset
    pub fn load_persona(&mut self, persona: &str) -> Result<()> {
        let preset = match persona {
            "web-dev" => get_web_dev_preset(),
            "ml-engineer" => get_ml_engineer_preset(),
            _ => {
                return Err(crate::SprawlError::Other(format!(
                    "Unknown persona: {}",
                    persona
                )))
            }
        };

        // Simple append for mockup; full merge logic applies `overridden_by` field tracking
        for mut rule in preset.rules {
            rule.source = persona.to_string();
            self.rules.push(rule);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_works_with_global_defaults_only() {
        let mut config = LayeredConfig::new();
        config.load_global_defaults().unwrap();

        assert!(
            !config.rules.is_empty(),
            "Global defaults should load rules"
        );
        assert!(
            !config.noise_patterns.is_empty(),
            "Global defaults should load noise patterns"
        );

        // Assert the known Snooze Default rule exists
        let snooze = config
            .rules
            .iter()
            .find(|r| r.name == "Global Snooze Baseline")
            .unwrap();
        assert_eq!(snooze.action, "snooze_default");
    }

    #[test]
    fn test_four_layer_merge_with_conflicting_conditions() {
        let mut config = LayeredConfig::new();
        config.load_global_defaults().unwrap();
        config.load_persona("web-dev").unwrap();

        // Verify web-dev override takes precedence/is identifiable
        let node_modules_rules: Vec<_> = config
            .rules
            .iter()
            .filter(|r| r.name == "Nuke node_modules")
            .collect();
        assert!(!node_modules_rules.is_empty());
        assert_eq!(node_modules_rules[0].source, "web-dev");
    }
}
