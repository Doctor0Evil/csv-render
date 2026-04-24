use crate::error::ValidationError;

#[derive(Debug, Clone, Copy)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Nanoseconds,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeMonotonicity {
    None,
    NonDecreasing,
    StrictIncreasing,
    PerGroup,
}

/// Column-level time contract compiled from schema data.
///
/// This struct is instantiated per timestamp column and reused as rows
/// are streamed through the validator.
#[derive(Debug)]
pub struct TimeConstraint {
    pub unit: TimeUnit,
    pub min_epoch: u64,
    pub max_epoch: u64,
    pub monotonicity: TimeMonotonicity,
    prev: Option<u64>,
}

impl TimeConstraint {
    pub fn new(
        unit: TimeUnit,
        min_epoch: u64,
        max_epoch: u64,
        monotonicity: TimeMonotonicity,
    ) -> Self {
        Self {
            unit,
            min_epoch,
            max_epoch,
            monotonicity,
            prev: None,
        }
    }

    /// Validate a single timestamp cell.
    ///
    /// `row` and `col` are 1-based data coordinates.
    pub fn validate(
        &mut self,
        row: usize,
        col: usize,
        raw: &str,
    ) -> Result<(), ValidationError> {
        let trimmed = raw.trim();

        let ts = trimmed.parse::<u64>().map_err(|_| ValidationError::Semantic {
            code: "T2_TIME_PARSE",
            row,
            column: col,
            message: format!("timestamp '{}' is not a valid u64", trimmed),
            details: serde_json::json!({
                "value": trimmed,
            }),
        })?;

        // Range check
        if ts < self.min_epoch || ts > self.max_epoch {
            return Err(ValidationError::Semantic {
                code: "T2_TIME_RANGE_VIOLATION",
                row,
                column: col,
                message: format!(
                    "timestamp {} is outside allowed range [{}..={}]",
                    ts, self.min_epoch, self.max_epoch
                ),
                details: serde_json::json!({
                    "value": ts,
                    "min_epoch": self.min_epoch,
                    "max_epoch": self.max_epoch,
                }),
            });
        }

        // Monotonicity check
        match self.monotonicity {
            TimeMonotonicity::None => {
                self.prev = Some(ts);
            }
            TimeMonotonicity::NonDecreasing | TimeMonotonicity::StrictIncreasing => {
                if let Some(prev) = self.prev {
                    let ok = match self.monotonicity {
                        TimeMonotonicity::NonDecreasing => ts >= prev,
                        TimeMonotonicity::StrictIncreasing => ts > prev,
                        _ => true,
                    };
                    if !ok {
                        return Err(ValidationError::Semantic {
                            code: "T2_TIME_MONOTONICITY_VIOLATION",
                            row,
                            column: col,
                            message: format!(
                                "timestamp {} violates {:?} ordering (previous {})",
                                ts, self.monotonicity, prev
                            ),
                            details: serde_json::json!({
                                "value": ts,
                                "previous": prev,
                                "monotonicity": format!("{:?}", self.monotonicity),
                            }),
                        });
                    }
                }
                self.prev = Some(ts);
            }
            TimeMonotonicity::PerGroup => {
                // Group-wise monotonicity must be implemented by a separate
                // higher-level structure that manages `prev` per group key.
                self.prev = Some(ts);
            }
        }

        Ok(())
    }
}
