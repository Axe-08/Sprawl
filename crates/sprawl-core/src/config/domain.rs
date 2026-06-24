use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TriageCondition {
    pub predicates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepRule {
    pub name: String,
    pub description: String,
    pub condition: TriageCondition,
    pub action: String,
    pub source: String, // Tracks where this rule came from (e.g. "Global Defaults", "web-dev")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoisePattern {
    pub name: String,
    pub pattern: String,
    pub source: String,
}

impl NoisePattern {
    pub fn compile_regex(&self) -> Result<regex::Regex, crate::SprawlError> {
        regex::Regex::new(&self.pattern)
            .map_err(|e| crate::SprawlError::Other(format!("Invalid regex in noise pattern '{}': {}", self.name, e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_regex_in_noise_pattern_errors_at_load_time() {
        let pattern = NoisePattern {
            name: "Invalid regex".to_string(),
            pattern: "[unclosed_bracket".to_string(),
            source: "Test".to_string(),
        };

        let err = pattern.compile_regex();
        assert!(err.is_err(), "Expected regex compilation to fail for invalid pattern");
        
        let err_msg = err.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid regex in noise pattern"), "Error message should clearly indicate regex failure");
    }

    #[test]
    fn test_valid_regex_compiles() {
        let pattern = NoisePattern {
            name: "Valid regex".to_string(),
            pattern: "^node_modules$".to_string(),
            source: "Test".to_string(),
        };

        assert!(pattern.compile_regex().is_ok(), "Valid regex should compile successfully");
    }
}
