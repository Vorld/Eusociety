//! Defines the data structures used for configuring the simulation and transport layers.
//! These structs are typically deserialized from a JSON configuration file (e.g., `config.json`).

use serde::{Deserialize, Serialize};

/// Root configuration structure encompassing all settings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Simulation-specific parameters.
    pub simulation: SimulationConfig,
    /// Transport (serialization and sending) parameters.
    pub transport: TransportConfig,
}

/// Simulation-specific configuration parameters.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimulationConfig {
    /// The number of particles to simulate.
    pub particle_count: usize,
    /// The width and height of the simulation world boundaries.
    pub world_dimensions: (f32, f32),
    /// The maximum speed a particle can reach.
    pub max_velocity: f32,
    /// Factor controlling the magnitude of random velocity changes per frame.
    pub velocity_randomization_factor: f32,
    /// Factor applied to velocity each frame to simulate drag/friction (0.0 to 1.0).
    pub velocity_damping_factor: f32, 
    /// How particles behave when they hit the world boundaries.
    pub boundary_behavior: BoundaryBehavior,
    /// Target frame rate for the simulation loop.
    pub frame_rate: u32,
}

/// Transport-specific configuration parameters.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransportConfig {
    /// Configuration for the data serializer.
    pub serializer: SerializerConfig,
    /// Configuration for the data sender.
    pub sender: SenderConfig,
    /// Enable delta compression (send only changed particle data)? (Default: false)
    /// Primarily effective with `OptimizedBinarySerializer` and `WebSocketSender`.
    pub delta_compression: Option<bool>, 
    /// Movement distance threshold for delta compression. Only particles moving more
    /// than this distance since the last sent state will be included. (Default: 0.1)
    pub delta_threshold: Option<f32>,
    /// Configuration for enabling and tuning parallel serialization.
    /// Primarily effective with `OptimizedBinarySerializer`.
    pub parallel_serialization: Option<ParallelSerializationConfig>,
    /// Frequency (in frames) to log transport performance metrics (serialization time, send time, data size).
    /// `Some(0)` logs every frame, `Some(N)` logs every N frames, `None` disables logging. (Default: None)
    pub log_frequency: Option<u32>, 
}

/// Configuration options for parallel serialization.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelSerializationConfig {
    /// Enable parallel serialization using Rayon? (Default: true for OptimizedBinarySerializer)
    pub enabled: bool,
    /// Minimum number of particles required to trigger parallel serialization. (Default: 50000)
    pub threshold: Option<usize>,
    /// Number of threads to use for the Rayon pool (0 = automatic). (Default: 0)
    pub thread_count: Option<usize>,
}

// --- Serializer Configuration ---

/// Configuration specific to the JSON serializer. (Currently empty, placeholder for future options).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonSerializerConfig {
    // Example: pub pretty_print: Option<bool>,
}

/// Configuration specific to the Binary (bincode) serializer. (Currently empty, placeholder for future options).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BinarySerializerConfig {
    // Example: pub endianness: Option<String>, // "Big" or "Little"
}

/// Configuration specific to the Null serializer (no options needed).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NullSerializerConfig {}

/// Enum defining the active serializer type and its specific configuration options.
/// Uses `serde(tag = "serializer_type", content = "options")` for clear JSON representation.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "serializer_type", content = "options")] 
pub enum SerializerConfig {
    /// Use JSON serialization.
    Json(JsonSerializerConfig),
    /// Use Bincode binary serialization.
    Binary(BinarySerializerConfig),
    /// Use a null serializer (no serialization occurs).
    Null(NullSerializerConfig), 
}

// --- Sender Configuration ---

/// Configuration specific to the File sender.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSenderConfig {
    /// Path to the output file.
    pub output_path: String,
    /// Frequency (in frames) to write data to the file (must be > 0).
    pub output_frequency: u32,
}

/// Configuration specific to the WebSocket sender.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebSocketSenderConfig {
    /// Network address and port to bind the WebSocket server to (e.g., "127.0.0.1:9001").
    pub websocket_address: String,
    /// Frequency (in frames) to send updates to connected clients (must be > 0).
    pub update_frequency: u32,
}

/// Configuration specific to the Null sender (no options needed).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NullSenderConfig {}

/// Enum defining the active sender type and its specific configuration options.
/// Uses `serde(tag = "sender_type", content = "options")` for clear JSON representation.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "sender_type", content = "options")] 
pub enum SenderConfig {
    /// Send data to a file.
    File(FileSenderConfig),
    /// Send data via a WebSocket server.
    WebSocket(WebSocketSenderConfig),
    /// Use a null sender (data is not sent anywhere).
    Null(NullSenderConfig), 
}

// --- Other Enums ---

/// Defines how particles behave when they reach the simulation world boundaries.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BoundaryBehavior {
    /// Particles wrap around to the opposite side of the world.
    Wrap,
    /// Particles bounce off the boundaries, reversing their velocity component perpendicular to the boundary.
    Bounce,
}

// Note: The old SerializerType enum has been removed as SerializerConfig provides type information.
