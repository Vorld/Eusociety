use serde::{Deserialize, Serialize};

/// Root configuration structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub simulation: SimulationConfig,
    pub transport: TransportConfig,
}

/// Simulation-specific configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimulationConfig {
    pub particle_count: usize,
    pub world_dimensions: (f32, f32),
    pub max_initial_velocity: f32,
    pub velocity_randomization_factor: f32,
    pub boundary_behavior: BoundaryBehavior,
    pub frame_rate: u32,
}

/// Transport-specific configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransportConfig {
    pub serializer_type: SerializerType,
    pub output_path: String,
    pub output_frequency: u32,
}

/// Defines behavior when particles reach world boundaries
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BoundaryBehavior {
    Wrap,
    Bounce,
}

/// Available serialization formats
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SerializerType {
    Json,
    Binary,
}