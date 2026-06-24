#[derive(Debug, PartialEq)]
pub enum PredicateAST {
    GreaterThan(String, u32), // e.g., ("idle_days", 14)
}

pub fn parse_condition(cond: &str) -> crate::Result<PredicateAST> {
    let parts: Vec<&str> = cond.split_whitespace().collect();
    if parts.len() == 3 && parts[1] == ">" {
        let field = parts[0].to_string();
        let value = parts[2].parse::<u32>().map_err(|_| {
            crate::SprawlError::Other(format!("Invalid integer in condition: {}", parts[2]))
        })?;
        Ok(PredicateAST::GreaterThan(field, value))
    } else {
        Err(crate::SprawlError::Other(format!("Invalid condition format: {}", cond)))
    }
}

pub fn evaluate_predicate(ast: &PredicateAST, idle_days: u32) -> bool {
    match ast {
        PredicateAST::GreaterThan(field, val) => {
            if field == "idle_days" {
                idle_days > *val
            } else {
                false
            }
        }
    }
}
