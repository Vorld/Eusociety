//! Handles loading and validation of simulation configuration from a JSON file.

use std::fs;
use thiserror::Error;

// Re-export configuration types for easier access from other modules.
pub use self::types::{
    Config, SimulationConfig, TransportConfig, BoundaryBehavior,
    SerializerConfig, JsonSerializerConfig, BinarySerializerConfig, NullSerializerConfig, // Serializer types (Added Null)
    SenderConfig, FileSenderConfig, WebSocketSenderConfig, NullSenderConfig, // Sender types (Added Null)
    Point, PolygonWall // Added Point and PolygonWall for wall definitions
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

/// Provides methods for loading and validating the simulation `Config`.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Loads configuration from a JSON file at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the configuration file (e.g., "config.json").
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if the file cannot be read, JSON parsing fails,
    /// or the configuration fails validation checks.
    pub fn from_file(path: &str) -> Result<Config, ConfigError> {
        let file_content = fs::read_to_string(path)
            .map_err(ConfigError::FileReadError)?;

        let config: Config = serde_json::from_str(&file_content)
            .map_err(ConfigError::JsonParseError)?;

        // Perform validation after loading
        Self::validate(&config)?;

        Ok(config)
    }

    /// Validates the loaded `Config` struct.
    ///
    /// Checks include:
    /// - Positive particle count.
    /// - Positive world dimensions.
    /// - Positive frame rate.
    /// - Sender-specific frequency validation (if applicable).
    ///
    /// # Arguments
    ///
    /// * `config` - A reference to the `Config` struct to validate.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ValidationError` if any validation check fails.
    pub fn validate(config: &Config) -> Result<(), ConfigError> {
        // --- Simulation Config Validation ---
        if config.simulation.particle_count == 0 {
            return Err(ConfigError::ValidationError(
                "Particle count must be greater than 0".to_string()
            ));
        }

        let (width, height) = config.simulation.world_dimensions;
        if width <= 0.0 || height <= 0.0 {
            return Err(ConfigError::ValidationError(
                "World dimensions must be positive".to_string()
            ));
        }

        if config.simulation.frame_rate == 0 {
            return Err(ConfigError::ValidationError(
                "Frame rate must be greater than 0".to_string()
            ));
        }

        // --- Transport Config Validation ---
        match &config.transport.sender { 
            SenderConfig::File(file_config) => { 
                if file_config.output_frequency == 0 {
                    return Err(ConfigError::ValidationError(
                        "File sender output_frequency must be greater than 0".to_string()
                    ));
                }
            }
            SenderConfig::WebSocket(ws_config) => { 
                if ws_config.update_frequency == 0 {
                    return Err(ConfigError::ValidationError(
                        "WebSocket sender update_frequency must be greater than 0".to_string()
                    ));
                }
                // TODO: Consider adding basic validation for websocket_address format (e.g., contains ':')
            }
            SenderConfig::Null(_) => { 
                // No frequency validation needed for Null sender
            }
        }

        // TODO: Add validation for delta_threshold if delta_compression is true? (e.g., must be positive)
        // TODO: Add validation for parallel_serialization thresholds/counts? (e.g., must be non-negative)

        Ok(())
    }
}
