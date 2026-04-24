use std::collections::HashMap;
use std::path::Path;

use crate::error::ValidationError;
use crate::schema::Schema;

/// Identifier for a logical shard role inside an invariant context
/// (e.g., "plan", "titan_metrics").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShardRole(pub String);

/// Abstract view of a typed row from any shard.
/// In a real system this will be backed by ALN-typed accessors or
/// strongly typed Rust structs per doctype.
#[derive(Debug, Clone)]
pub struct ShardRow {
    pub key: String,                     // e.g., basin_id
    pub fields: HashMap<String, f64>,    // normalized risk coordinates & bounds
}

/// Streaming iterator over rows from a single shard.
pub trait ShardStream {
    fn next_row(&mut self) -> Result<Option<ShardRow>, ValidationError>;
}

/// Factory to build ShardStreams for a given doctype + file.
/// This lets CrossShardContext be decoupled from CSV vs ALN backends.
pub trait ShardStreamFactory {
    fn open_shard_stream(
        &self,
        doctype: &str,
        path: &Path,
        key_column: &str,
    ) -> Result<Box<dyn ShardStream + Send>, ValidationError>;
}

/// A Lyapunov kernel over joined shard tuples for one key.
pub trait LyapunovKernel {
    /// Check non-expansion for a single joined basin snapshot.
    ///
    /// `state_coords` are Titan normalized risk coordinates (rlatency, rjitter, rloss, roh, ...).
    /// `corridor_bounds` are the corresponding Titan corridor bounds.
    fn check_nonexpansion(
        &self,
        key: &str,
        state_coords: &HashMap<String, f64>,
        corridor_bounds: &HashMap<String, f64>,
    ) -> Result<(), ValidationError>;
}

/// A cross-plane envelope kernel enforcing
/// titan_risk[r] <= min(water[r], sewer[r]) for each coordinate r.
pub trait CrossPlaneEnvelopeKernel {
    fn check_envelope(
        &self,
        key: &str,
        titan_risk: &HashMap<String, f64>,
        water_env: &HashMap<String, f64>,
        sewer_env: &HashMap<String, f64>,
    ) -> Result<(), ValidationError>;
}

/// Configuration for a single invariant context, compiled from ALN.
#[derive(Debug, Clone)]
pub struct ContextConfig {
    pub name: String,
    pub key_field: String,
    pub shards: HashMap<ShardRole, ShardConfig>,
}

#[derive(Debug, Clone)]
pub struct ShardConfig {
    pub doctype: String,
    pub path: String,
    pub key_column: String,
}

/// Runtime context that can stream and join multiple shards
/// by a common key (e.g., basin_id) and apply cross-shard invariants.
pub struct CrossShardContext<'a> {
    pub config: ContextConfig,
    factory: &'a dyn ShardStreamFactory,
}

impl<'a> CrossShardContext<'a> {
    pub fn new(config: ContextConfig, factory: &'a dyn ShardStreamFactory) -> Self {
        Self { config, factory }
    }

    /// Execute the Lyapunov non-expansion invariant across all basins.
    ///
    /// This corresponds to the `titan_lyapunov_nonexpansion` invariant in ALN.
    pub fn enforce_lyapunov_nonexpansion(
        &self,
        lyapunov_kernel: &dyn LyapunovKernel,
    ) -> Result<(), ValidationError> {
        let plan_cfg = self
            .config
            .shards
            .get(&ShardRole("plan".to_string()))
            .ok_or_else(|| ValidationError::Semantic {
                row: 0,
                column: 0,
                message: "context missing shard 'plan'".to_string(),
            })?;

        let delay_cfg = self
            .config
            .shards
            .get(&ShardRole("titan_delay".to_string()))
            .ok_or_else(|| ValidationError::Semantic {
                row: 0,
                column: 0,
                message: "context missing shard 'titan_delay'".to_string(),
            })?;

        let metrics_cfg = self
            .config
            .shards
            .get(&ShardRole("titan_metrics".to_string()))
            .ok_or_else(|| ValidationError::Semantic {
                row: 0,
                column: 0,
                message: "context missing shard 'titan_metrics'".to_string(),
            })?;

        let mut plan_stream = self.factory.open_shard_stream(
            &plan_cfg.doctype,
            Path::new(&plan_cfg.path),
            &plan_cfg.key_column,
        )?;
        let mut delay_stream = self.factory.open_shard_stream(
            &delay_cfg.doctype,
            Path::new(&delay_cfg.path),
            &delay_cfg.key_column,
        )?;
        let mut metrics_stream = self.factory.open_shard_stream(
            &metrics_cfg.doctype,
            Path::new(&metrics_cfg.path),
            &metrics_cfg.key_column,
        )?;

        // For simplicity, we build key -> row maps once; for very large shards
        // you would move to streaming joins keyed by sorted order.
        let plan_map = collect_by_key(&mut plan_stream)?;
        let delay_map = collect_by_key(&mut delay_stream)?;
        let metrics_map = collect_by_key(&mut metrics_stream)?;

        for (key, metrics_row) in metrics_map.iter() {
            let delay_row = match delay_map.get(key) {
                Some(r) => r,
                None => {
                    return Err(ValidationError::Semantic {
                        row: 0,
                        column: 0,
                        message: format!("missing titan_delay row for key {}", key),
                    })
                }
            };
            // Plan row is logically part of the context; you can decide whether
            // to require it strictly or tolerate missing entries.
            if !plan_map.contains_key(key) {
                return Err(ValidationError::Semantic {
                    row: 0,
                    column: 0,
                    message: format!("missing RegionLinkPlan row for key {}", key),
                });
            }

            let mut state_coords = HashMap::new();
            for name in ["rlatency", "rjitter", "rloss", "roh"] {
                if let Some(v) = metrics_row.fields.get(name) {
                    state_coords.insert(name.to_string(), *v);
                }
            }

            let mut corridor_bounds = HashMap::new();
            for (src, dst) in [
                ("max_delay_ms", "max_delay_ms"),
                ("max_jitter_ms", "max_jitter_ms"),
                ("max_loss", "max_loss"),
            ] {
                if let Some(v) = delay_row.fields.get(src) {
                    corridor_bounds.insert(dst.to_string(), *v);
                }
            }

            lyapunov_kernel.check_nonexpansion(key, &state_coords, &corridor_bounds)?;
        }

        Ok(())
    }

    /// Execute the cross-plane envelope containment invariant across all basins.
    ///
    /// This corresponds to the `cross_plane_envelope_containment` invariant in ALN.
    pub fn enforce_cross_plane_envelope(
        &self,
        envelope_kernel: &dyn CrossPlaneEnvelopeKernel,
    ) -> Result<(), ValidationError> {
        let metrics_cfg = self
            .config
            .shards
            .get(&ShardRole("titan_metrics".to_string()))
            .ok_or_else(|| ValidationError::Semantic {
                row: 0,
                column: 0,
                message: "context missing shard 'titan_metrics'".to_string(),
            })?;

        let water_cfg = self
            .config
            .shards
            .get(&ShardRole("water_corridor".to_string()))
            .ok_or_else(|| ValidationError::Semantic {
                row: 0,
                column: 0,
                message: "context missing shard 'water_corridor'".to_string(),
            })?;

        let sewer_cfg = self
            .config
            .shards
            .get(&ShardRole("sewer_corridor".to_string()))
            .ok_or_else(|| ValidationError::Semantic {
                row: 0,
                column: 0,
                message: "context missing shard 'sewer_corridor'".to_string(),
            })?;

        let mut metrics_stream = self.factory.open_shard_stream(
            &metrics_cfg.doctype,
            Path::new(&metrics_cfg.path),
            &metrics_cfg.key_column,
        )?;
        let mut water_stream = self.factory.open_shard_stream(
            &water_cfg.doctype,
            Path::new(&water_cfg.path),
            &water_cfg.key_column,
        )?;
        let mut sewer_stream = self.factory.open_shard_stream(
            &sewer_cfg.doctype,
            Path::new(&sewer_cfg.path),
            &sewer_cfg.key_column,
        )?;

        let metrics_map = collect_by_key(&mut metrics_stream)?;
        let water_map = collect_by_key(&mut water_stream)?;
        let sewer_map = collect_by_key(&mut sewer_stream)?;

        for (key, titan_row) in metrics_map.iter() {
            let water_row = match water_map.get(key) {
                Some(r) => r,
                None => {
                    return Err(ValidationError::Semantic {
                        row: 0,
                        column: 0,
                        message: format!("missing WATER corridor row for key {}", key),
                    })
                }
            };
            let sewer_row = match sewer_map.get(key) {
                Some(r) => r,
                None => {
                    return Err(ValidationError::Semantic {
                        row: 0,
                        column: 0,
                        message: format!("missing SEWER corridor row for key {}", key),
                    })
                }
            };

            let mut titan_risk = HashMap::new();
            for name in ["rlatency", "rjitter", "rloss", "roh"] {
                if let Some(v) = titan_row.fields.get(name) {
                    titan_risk.insert(name.to_string(), *v);
                }
            }

            let mut water_env = HashMap::new();
            let mut sewer_env = HashMap::new();
            for name in ["rlatency_max", "rjitter_max", "rloss_max", "roh_max"] {
                if let Some(v) = water_row.fields.get(name) {
                    water_env.insert(name.to_string(), *v);
                }
                if let Some(v) = sewer_row.fields.get(name) {
                    sewer_env.insert(name.to_string(), *v);
                }
            }

            envelope_kernel.check_envelope(key, &titan_risk, &water_env, &sewer_env)?;
        }

        Ok(())
    }
}

fn collect_by_key(
    stream: &mut dyn ShardStream,
) -> Result<HashMap<String, ShardRow>, ValidationError> {
    let mut map = HashMap::new();
    while let Some(row) = stream.next_row()? {
        map.insert(row.key.clone(), row);
    }
    Ok(map)
}
