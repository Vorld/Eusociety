use std::fs;
use thiserror::Error;

// Re-export types
pub use self::types::{Config, SimulationConfig, TransportConfig, BoundaryBehavior, SerializerType, SenderType};
mod types;

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
        
        // Validate output frequency
        if config.transport.output_frequency == 0 {
            return Err(ConfigError::ValidationError(
                "Output frequency must be greater than 0".to_string()
            ));
        }
        
        Ok(())
    }
}