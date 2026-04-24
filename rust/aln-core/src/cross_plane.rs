// rust/aln-core/src/cross_plane.rs
use crate::normalization::NormalizationProfile;

#[derive(Debug)]
pub struct CorridorBounds {
    pub coordinate: String,
    pub cap: f64,
}

#[derive(Debug)]
pub struct PlaneCorridor {
    pub plane: String, // "WATER", "SEWER", "TITAN"
    pub bounds: Vec<CorridorBounds>,
}

pub fn corridor_inclusion(
    titan: &PlaneCorridor,
    others: &[PlaneCorridor],
) -> Result<(), String> {
    for t in &titan.bounds {
        for other in others {
            if let Some(o) = other.bounds.iter().find(|o| o.coordinate == t.coordinate) {
                if t.cap > o.cap {
                    return Err(format!(
                        "Titan widens corridor for {}: titan cap {} > {} cap {} on plane {}",
                        t.coordinate, t.cap, other.plane, o.cap, other.plane
                    ));
                }
            }
        }
    }
    Ok(())
}
