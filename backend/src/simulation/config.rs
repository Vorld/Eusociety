use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// This module contains the configuration structs for the simulation.

// The WorldConfig struct contains the configuration for the world boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub width: f32,
    pub height: f32,
    pub boundary_mode: BoundaryMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoundaryMode {
    Wrap,
    Bounce,
    Kill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub world: WorldConfig,
    pub entity_configs: Vec<EntityConfig>,
    pub field_configs: Vec<FieldConfig>,
    pub tick_rate_ms: u64,
    pub broadcast_rate: u32,
    pub max_chunk_size: usize, 
    pub batch_size: usize,  
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityConfig {
    pub entity_type: String,
    pub count: usize,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfig {
    pub field_type: String,
    pub resolution: usize,
    pub decay_rate: f32,
    pub diffusion_rate: f32,
    pub properties: HashMap<String, serde_json::Value>,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            width: 6000.0,
            height: 6000.0,
            boundary_mode: BoundaryMode::Bounce,
        }
    }
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            world: WorldConfig::default(),
            entity_configs: vec![
                EntityConfig {
                    entity_type: "particle".to_string(),
                    count: 1000,
                    properties: HashMap::new(),
                }
            ],
            field_configs: vec![],
            tick_rate_ms: 10,
            broadcast_rate: 1,
            max_chunk_size: 65536,
            batch_size: 4096, 
        }
    }
}