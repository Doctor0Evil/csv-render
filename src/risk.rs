// src/risk.rs
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskRawType {
    U64,
    F64,
    Percent01,
    Percent100,
    Ordinal(u8), // number of categories
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskNormalizerKind {
    Linear,
    LogClamp,
    LogitClamp,
    ZScoreCdf,
    OrdinalLadder,
    Identity, // for Normalized01
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskMonotonicity {
    None,
    NonDecreasing,
    StrictIncreasing,
}

#[derive(Debug, Clone)]
pub struct RiskProfile {
    pub id: String,
    pub raw_type: RiskRawType,
    pub kind: RiskNormalizerKind,
    pub domain_min: f64,
    pub domain_max: f64,
    pub clamp_min: Option<f64>,
    pub clamp_max: Option<f64>,
    pub monotonicity: RiskMonotonicity,
    pub normalized01: bool,
}

#[derive(Debug, Clone)]
pub struct RiskState {
    prev_normalized: Option<f64>,
}

impl RiskProfile {
    pub fn normalize_raw(&self, raw: f64) -> Result<f64, ValidationError> {
        // domain check
        if raw.is_nan() {
            return Err(ValidationError::semantic(
                "T2RISK_NAN",
                format!("risk value {} is NaN", raw),
            ));
        }
        if raw < self.domain_min || raw > self.domain_max {
            return Err(ValidationError::semantic(
                "T2RISK_OUT_OF_RANGE",
                format!(
                    "risk value {} outside [{}, {}]",
                    raw, self.domain_min, self.domain_max
                ),
            ));
        }

        let mut x = raw;
        if let Some(lo) = self.clamp_min {
            if x < lo {
                x = lo;
            }
        }
        if let Some(hi) = self.clamp_max {
            if x > hi {
                x = hi;
            }
        }

        let y = match self.kind {
            RiskNormalizerKind::Identity => x,
            RiskNormalizerKind::Linear => {
                if (self.domain_max - self.domain_min).abs() < f64::EPSILON {
                    return Err(ValidationError::semantic(
                        "T2RISK_BAD_LINEAR_DOMAIN",
                        "linear normalization with zero-width domain".to_string(),
                    ));
                }
                (x - self.domain_min) / (self.domain_max - self.domain_min)
            }
            RiskNormalizerKind::LogClamp => {
                if x <= 0.0 {
                    0.0
                } else {
                    let xmin = if self.domain_min <= 0.0 {
                        f64::MIN_POSITIVE
                    } else {
                        self.domain_min
                    };
                    let xmax = self.domain_max;
                    let num = (x / xmin).ln();
                    let den = (xmax / xmin).ln();
                    if den <= 0.0 {
                        return Err(ValidationError::semantic(
                            "T2RISK_BAD_LOG_DOMAIN",
                            "invalid log domain".to_string(),
                        ));
                    }
                    let r = num / den;
                    r.clamp(0.0, 1.0)
                }
            }
            RiskNormalizerKind::LogitClamp => {
                let eps = 1e-12;
                let p = x.clamp(eps, 1.0 - eps);
                let g = (p / (1.0 - p)).ln();
                let l = self.domain_min;
                let u = self.domain_max;
                if u <= l {
                    return Err(ValidationError::semantic(
                        "T2RISK_BAD_LOGIT_DOMAIN",
                        "upper_logit <= lower_logit".to_string(),
                    ));
                }
                ((g.clamp(l, u) - l) / (u - l)).clamp(0.0, 1.0)
            }
            RiskNormalizerKind::ZScoreCdf => {
                // Simple erf-based approximation or call into a math crate
                crate::math::phi(x)
            }
            RiskNormalizerKind::OrdinalLadder => {
                // Here domain_min, domain_max encode category index bounds,
                // and we assume a simple linear ladder.
                let cats = (self.domain_max - self.domain_min).abs().max(1.0);
                let idx = x - self.domain_min;
                (idx / cats).clamp(0.0, 1.0)
            }
        };

        if self.normalized01 {
            if y < -1e-9 || y > 1.0 + 1e-9 {
                return Err(ValidationError::semantic(
                    "T2RISK_NOT_NORMALIZED01",
                    format!("normalized risk {} not in [0,1]", y),
                ));
            }
        }

        Ok(y)
    }
}

impl RiskState {
    pub fn new() -> Self {
        RiskState { prev_normalized: None }
    }

    pub fn validate_monotone(
        &mut self,
        profile: &RiskProfile,
        row: usize,
        col: usize,
        y: f64,
    ) -> Result<(), ValidationError> {
        if let Some(prev) = self.prev_normalized {
            let ok = match profile.monotonicity {
                RiskMonotonicity::None => true,
                RiskMonotonicity::NonDecreasing => y >= prev,
                RiskMonotonicity::StrictIncreasing => y > prev,
            };
            if !ok {
                return Err(ValidationError::semantic(
                    "T2RISK_MONOTONICITY",
                    format!(
                        "risk violates {:?} monotonicity at row {}, col {} (prev={}, curr={})",
                        profile.monotonicity, row, col, prev, y
                    ),
                ));
            }
        }
        self.prev_normalized = Some(y);
        Ok(())
    }
}
