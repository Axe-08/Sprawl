use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Predicate {
    pub operand: String,
    pub operator: String,
    pub value: u64,
}

impl Predicate {
    pub fn evaluate(&self, context_value: u64) -> bool {
        match self.operator.as_str() {
            ">" => context_value > self.value,
            "<" => context_value < self.value,
            ">=" => context_value >= self.value,
            "<=" => context_value <= self.value,
            "==" => context_value == self.value,
            "!=" => context_value != self.value,
            _ => false,
        }
    }
}

// Implement parsing logic based on TRD spec: `idle_days > 14` -> Predicate
impl std::str::FromStr for Predicate {
    type Err = crate::SprawlError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(crate::SprawlError::Other(format!("Invalid predicate syntax: {}", s)));
        }

        let operand = parts[0].to_string();
        let operator = parts[1].to_string();
        
        let value = parts[2].parse::<u64>().map_err(|_| {
            crate::SprawlError::Other(format!("Invalid predicate value: {}", parts[2]))
        })?;

        match operator.as_str() {
            ">" | "<" | ">=" | "<=" | "==" | "!=" => Ok(Predicate { operand, operator, value }),
            _ => Err(crate::SprawlError::Other(format!("Invalid predicate operator: {}", operator))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_predicate_parses_all_operands_and_operators() {
        let cases = vec![
            ("idle_days > 14", ">", 14),
            ("disk_size_mb >= 500", ">=", 500),
            ("cache_age_days < 7", "<", 7),
            ("build_artifacts <= 100", "<=", 100),
            ("count == 0", "==", 0),
            ("errors != 5", "!=", 5),
        ];

        for (expr, op, val) in cases {
            let p = Predicate::from_str(expr).expect("Failed to parse valid predicate");
            assert_eq!(p.operator, op);
            assert_eq!(p.value, val);
        }
    }

    #[test]
    fn test_predicate_evaluation() {
        let p = Predicate::from_str("idle_days > 14").unwrap();
        assert!(p.evaluate(15));
        assert!(!p.evaluate(14));
        assert!(!p.evaluate(10));
    }

    #[test]
    fn test_predicate_rejects_or_and_parens() {
        // Spec strictly says OR/parens are not supported
        let invalid = vec![
            "idle_days > 14 OR size > 100",
            "(idle_days > 14)",
            "idle_days > 14 AND size > 100",
        ];

        for expr in invalid {
            assert!(Predicate::from_str(expr).is_err(), "Expected failure for complex expressions: {}", expr);
        }
    }
}
