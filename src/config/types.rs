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
    pub serializer: SerializerConfig,
    pub sender: SenderConfig,
    pub delta_compression: Option<bool>, // Keep delta compression general for now
    pub delta_threshold: Option<f32>,
    pub parallel_serialization: Option<ParallelSerializationConfig>,
    pub log_frequency: Option<u32>, // Frequency to log transport performance (every N frames, 0=always, None=never)
}

/// Configuration for parallel serialization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelSerializationConfig {
    pub enabled: bool,
    pub threshold: Option<usize>,
    pub thread_count: Option<usize>,
}

// --- Serializer Configuration ---

/// Configuration specific to the JSON serializer (currently empty)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonSerializerConfig {
    // Add JSON-specific options here later if needed
}

/// Configuration specific to the Binary serializer (currently empty)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BinarySerializerConfig {
    // Add Binary-specific options here later if needed
}

/// Configuration specific to the Null serializer (empty)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NullSerializerConfig {}

/// Enum defining the serializer type and its specific configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "serializer_type", content = "options")] // Nest options
pub enum SerializerConfig {
    Json(JsonSerializerConfig),
    Binary(BinarySerializerConfig),
    Null(NullSerializerConfig), // Associate Null variant with the empty struct
}

// --- Sender Configuration ---

/// Configuration specific to the File sender
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSenderConfig {
    pub output_path: String,
    pub output_frequency: u32,
}

/// Configuration specific to the WebSocket sender
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebSocketSenderConfig {
    pub websocket_address: String,
    pub update_frequency: u32,
}

/// Configuration specific to the Null sender (empty)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NullSenderConfig {}

/// Enum defining the sender type and its specific configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "sender_type", content = "options")] // Nest options
pub enum SenderConfig {
    File(FileSenderConfig),
    WebSocket(WebSocketSenderConfig),
    Null(NullSenderConfig), // Associate Null variant with the empty struct
}

// --- Other Enums ---

/// Defines behavior when particles reach world boundaries
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BoundaryBehavior {
    Wrap,
    Bounce,
}

// Note: The old SerializerType enum has been removed.
