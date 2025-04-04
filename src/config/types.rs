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
    pub max_velocity: f32,
    pub velocity_randomization_factor: f32,
    pub velocity_damping_factor: f32, // Added damping factor
    pub boundary_behavior: BoundaryBehavior,
    pub frame_rate: u32,
}

/// Transport-specific configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransportConfig {
    pub serializer_type: SerializerType,
    pub sender_type: SenderType,
    pub output_path: String,
    pub output_frequency: u32,
    pub websocket_address: Option<String>,
    pub delta_compression: Option<bool>,
    pub update_frequency: Option<u32>,
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

/// Available sender types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SenderType {
    File,
    WebSocket,
}
