// Filename: rust/aln-kernel/src/risk_scalar.rs

use crate::error::ValidationError;

#[derive(Debug, Clone, Copy)]
pub enum NormalizationKind {
    Linear01Clamp,
    Logit01Clamp,
    Custom(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub enum RiskModifier {
    Normalized01,
    Capped(f64),
    MonotoneIncreasing,
}

#[derive(Debug, Clone)]
pub struct RiskScalarContract {
    pub normalization_id: NormalizationKind,
    pub risk_coordinate:  &'static str,
    pub corridor_min:    f64,
    pub corridor_max:    f64,
    pub modifiers:       Vec<RiskModifier>,
}

impl RiskScalarContract {
    pub fn normalize(&self, raw: f64) -> Result<f64, ValidationError> {
        // 1. Range check against corridor
        if raw < self.corridor_min || raw > self.corridor_max {
            return Err(ValidationError::semantic(
                "TITAN_RISK_RANGE",
                format!(
                    "raw value {} for {} is outside [{}, {}]",
                    raw, self.risk_coordinate, self.corridor_min, self.corridor_max
                ),
            ));
        }

        // 2. Apply normalization
        let mut y = match self.normalization_id {
            NormalizationKind::Linear01Clamp => {
                let denom = self.corridor_max - self.corridor_min;
                if denom <= 0.0 {
                    return Err(ValidationError::semantic(
                        "TITAN_RISK_BAD_CORRIDOR",
                        format!(
                            "degenerate corridor for {}: min={} max={}",
                            self.risk_coordinate, self.corridor_min, self.corridor_max
                        ),
                    ));
                }
                let z = (raw - self.corridor_min) / denom;
                z.clamp(0.0, 1.0)
            }
            NormalizationKind::Logit01Clamp => {
                let denom = self.corridor_max - self.corridor_min;
                if denom <= 0.0 {
                    return Err(ValidationError::semantic(
                        "TITAN_RISK_BAD_CORRIDOR",
                        format!(
                            "degenerate corridor for {}: min={} max={}",
                            self.risk_coordinate, self.corridor_min, self.corridor_max
                        ),
                    ));
                }
                let z = (raw - self.corridor_min) / denom;
                let z = z.clamp(0.0 + f64::EPSILON, 1.0 - f64::EPSILON);
                (z / (1.0 - z)).ln()
            }
            NormalizationKind::Custom(id) => {
                return Err(ValidationError::semantic(
                    "TITAN_RISK_NORMALIZATION_UNIMPLEMENTED",
                    format!("custom normalization '{}' not implemented", id),
                ));
            }
        };

        // 3. Enforce modifiers
        for m in &self.modifiers {
            match *m {
                RiskModifier::Normalized01 => {
                    if !(0.0 <= y && y <= 1.0) {
                        return Err(ValidationError::semantic(
                            "TITAN_RISK_NORMALIZED01",
                            format!(
                                "normalized {} for {} is {}, outside [0,1]",
                                self.normalization_id_str(),
                                self.risk_coordinate,
                                y
                            ),
                        ));
                    }
                }
                RiskModifier::Capped(cap) => {
                    if y > cap {
                        y = cap;
                    }
                }
                RiskModifier::MonotoneIncreasing => {
                    // Monotonicity must be enforced at a higher layer that
                    // remembers previous y for this coordinate and basin.
                }
            }
        }

        Ok(y)
    }

    fn normalization_id_str(&self) -> &'static str {
        match self.normalization_id {
            NormalizationKind::Linear01Clamp => "norm-linear-01-clamp",
            NormalizationKind::Logit01Clamp  => "norm-logit-01-clamp",
            NormalizationKind::Custom(s)     => s,
        }
    }
}
