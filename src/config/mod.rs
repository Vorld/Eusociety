use std::fs;
use thiserror::Error;

// Re-export types
pub use self::types::{
    Config, SimulationConfig, TransportConfig, BoundaryBehavior,
    SerializerConfig, JsonSerializerConfig, BinarySerializerConfig, // Serializer types
    SenderConfig, FileSenderConfig, WebSocketSenderConfig // Sender types
};
pub mod types; // Make the types module public

// Config error handling
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

// Config loader implementation
pub struct ConfigLoader;

impl ConfigLoader {
    pub fn from_file(path: &str) -> Result<Config, ConfigError> {
        let file_content = fs::read_to_string(path)
            .map_err(ConfigError::FileReadError)?;

        let config: Config = serde_json::from_str(&file_content)
            .map_err(ConfigError::JsonParseError)?;

        // Perform validation after loading
        Self::validate(&config)?;

        Ok(config)
    }

    pub fn validate(config: &Config) -> Result<(), ConfigError> {
        // Validate particle count
        if config.simulation.particle_count == 0 {
            return Err(ConfigError::ValidationError(
                "Particle count must be greater than 0".to_string()
            ));
        }

        // Validate world dimensions
        let (width, height) = config.simulation.world_dimensions;
        if width <= 0.0 || height <= 0.0 {
            return Err(ConfigError::ValidationError(
                "World dimensions must be positive".to_string()
            ));
        }

        // Validate frame rate
        if config.simulation.frame_rate == 0 {
            return Err(ConfigError::ValidationError(
                "Frame rate must be greater than 0".to_string()
            ));
        }

        // Validate sender-specific frequency (if applicable)
        match &config.transport.sender { // Match on reference
            SenderConfig::File(file_config) => { // Use match ergonomics (binds file_config by ref)
                if file_config.output_frequency == 0 {
                    return Err(ConfigError::ValidationError(
                        "File sender output_frequency must be greater than 0".to_string()
                    ));
                }
            }
            SenderConfig::WebSocket(ws_config) => { // Use match ergonomics (binds ws_config by ref)
                if ws_config.update_frequency == 0 {
                    return Err(ConfigError::ValidationError(
                        "WebSocket sender update_frequency must be greater than 0".to_string()
                    ));
                }
                // Optionally validate websocket_address format here if needed
            }
            SenderConfig::Null(_) => { // Use match ergonomics (ignores inner data)
                // No validation needed for Null sender
            }
        }

        // No validation needed for serializer options currently

        Ok(())
    }
}
