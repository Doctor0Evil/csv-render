use std::collections::HashSet;

use crate::error::ValidationError;

/// Contract for a compositional flag set, e.g. neurorights flags.
#[derive(Debug, Clone)]
pub struct FlagRules {
    pub allowed: HashSet<String>,
    pub exclusive: Vec<(String, String)>,
    pub requires: Vec<(String, String)>,
    pub min: Option<usize>,
    pub max: Option<usize>,
    pub contract_name: String,
}

impl FlagRules {
    /// Parse a raw string cell into a set of trimmed flag tokens.
    pub fn parse_flags(raw: &str, separator: char) -> HashSet<String> {
        raw.split(separator)
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect()
    }

    /// Validate a flag set against this contract.
    pub fn validate(
        &self,
        row: usize,
        col: usize,
        set: &HashSet<String>,
    ) -> Result<(), ValidationError> {
        // Membership
        for f in set {
            if !self.allowed.contains(f) {
                return Err(ValidationError::Semantic {
                    code: "T2_FLAG_UNKNOWN",
                    row,
                    column: col,
                    message: format!("flag '{}' is not allowed", f),
                    details: serde_json::json!({
                        "flag": f,
                        "allowed_contract": self.contract_name,
                    }),
                });
            }
        }

        // Mutual exclusivity
        for (a, b) in &self.exclusive {
            if set.contains(a) && set.contains(b) {
                return Err(ValidationError::Semantic {
                    code: "T2_FLAG_MUTUALLY_EXCLUSIVE",
                    row,
                    column: col,
                    message: format!(
                        "flags '{}' and '{}' may not co-occur in the same row",
                        a, b
                    ),
                    details: serde_json::json!({
                        "a": a,
                        "b": b,
                        "allowed_contract": self.contract_name,
                    }),
                });
            }
        }

        // Implications
        for (a, b) in &self.requires {
            if set.contains(a) && !set.contains(b) {
                return Err(ValidationError::Semantic {
                    code: "T2_FLAG_MISSING_IMPLICATION",
                    row,
                    column: col,
                    message: format!("flag '{}' requires '{}'", a, b),
                    details: serde_json::json!({
                        "if": a,
                        "then": b,
                        "allowed_contract": self.contract_name,
                    }),
                });
            }
        }

        // Cardinality
        let n = set.len();
        if let Some(min) = self.min {
            if n < min {
                return Err(ValidationError::Semantic {
                    code: "T2_FLAG_CARDINALITY_MIN",
                    row,
                    column: col,
                    message: format!("{} flags < min {}", n, min),
                    details: serde_json::json!({
                        "count": n,
                        "min": min,
                        "allowed_contract": self.contract_name,
                    }),
                });
            }
        }
        if let Some(max) = self.max {
            if n > max {
                return Err(ValidationError::Semantic {
                    code: "T2_FLAG_CARDINALITY_MAX",
                    row,
                    column: col,
                    message: format!("{} flags > max {}", n, max),
                    details: serde_json::json!({
                        "count": n,
                        "max": max,
                        "allowed_contract": self.contract_name,
                    }),
                });
            }
        }

        Ok(())
    }
}
