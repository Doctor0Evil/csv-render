// rust/aln-core/src/normalization.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NormalizationProfile {
    pub id: String,
    pub risk_coordinate: String,
    pub cap: f64,
    pub clamp_min: f64,
    pub clamp_max: f64,
    pub monotone_invariant: bool,
}

impl NormalizationProfile {
    pub fn normalize(&self, raw: f64) -> f64 {
        let clamped = raw.clamp(self.clamp_min, self.clamp_max);
        let t = (clamped - self.clamp_min) / (self.clamp_max - self.clamp_min);
        let capped = t.min(self.cap);
        capped
    }
}

#[derive(Debug, Default)]
pub struct NormalizationRegistry {
    profiles: HashMap<String, NormalizationProfile>,
}

impl NormalizationRegistry {
    pub fn register(&mut self, profile: NormalizationProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    pub fn get(&self, id: &str) -> Option<&NormalizationProfile> {
        self.profiles.get(id)
    }

    pub fn validate_corridor(&self, coords: &[(&str, &str)]) -> Result<(), String> {
        // coords: (field_name, normalization_id)
        for (field, norm_id) in coords {
            let profile = self
                .get(norm_id)
                .ok_or_else(|| format!("Missing normalization profile {}", norm_id))?;
            if profile.risk_coordinate.is_empty() {
                return Err(format!("Field {} bound to profile {} without risk_coordinate",
                                   field, norm_id));
            }
        }
        Ok(())
    }
}
