use eusociety_core::{Position, DeltaTime}; // Add DeltaTime import
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

// --- Configuration Structs ---

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)] // Error if unknown fields are in JSON
pub struct EntityConfig {
    pub id: u32,
    // Using serde_json::Value allows flexibility for different component types initially.
    // In later milestones, we might use specific types or an enum.
    pub components: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct StartStateConfig {
    pub entities: Vec<EntityConfig>,
    // pub world: Option<HashMap<String, serde_json::Value>>, // Add later if needed
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SerializerConfig {
    #[serde(rename = "type")]
    pub type_: String, // e.g., "binary", "json"
    // pub options: Option<HashMap<String, serde_json::Value>>, // Add later if needed
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SenderConfig {
    #[serde(rename = "type")]
    pub type_: String, // e.g., "file", "websocket", "console"
    // Using Value for options allows flexibility (e.g., path for file, url for websocket)
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TransportConfig {
    pub serializer: SerializerConfig,
    pub sender: SenderConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SimulationConfig {
    pub threads: u32,
    pub fps: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub start_state: StartStateConfig,
    pub transport: TransportConfig,
    pub simulation: SimulationConfig,
    #[serde(default)]
    pub initial_resources: Option<HashMap<String, serde_json::Value>>,
}

// --- Error Handling ---

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse configuration file (JSON): {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Invalid configuration value: {0}")]
    ValidationError(String), // For semantic validation later
}

// --- Loading Function ---

/// Loads and parses the simulation configuration from a JSON file.
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
    let path_ref = path.as_ref();
    let config_str = fs::read_to_string(path_ref)?;
    let config: Config = serde_json::from_str(&config_str)?;

    // Basic validation (can be expanded)
    if config.simulation.fps == 0 {
        return Err(ConfigError::ValidationError(
            "Simulation FPS cannot be zero.".to_string(),
        ));
    }
    if config.simulation.threads == 0 {
         return Err(ConfigError::ValidationError(
            "Simulation threads cannot be zero.".to_string(),
        ));
    }
     if config.transport.sender.type_ == "file" && config.transport.sender.options.as_ref().map_or(true, |opts| !opts.contains_key("path")) {
         return Err(ConfigError::ValidationError(
            "File sender requires an 'options.path' field.".to_string(),
        ));
    }


    Ok(config)
}

// --- Helper Function for Component Parsing (Example) ---

/// Attempts to parse a Position component from the generic serde_json::Value.
/// This shows how the runner might extract specific components later.
pub fn parse_position_component(
    value: &serde_json::Value,
) -> Result<Position, ConfigError> {
    serde_json::from_value(value.clone()).map_err(|e| {
        ConfigError::ValidationError(format!("Failed to parse Position component: {}", e))
    })
}

/// Attempts to parse a DeltaTime resource from a serde_json::Value.
/// Expects a JSON object with a "delta_seconds" field.
pub fn parse_delta_time_resource(
    value: &serde_json::Value,
) -> Result<DeltaTime, ConfigError> {
    // Try to get delta_seconds as f32
    if let Some(seconds) = value.get("delta_seconds").and_then(|v| v.as_f64()) {
        // Convert to Duration and create DeltaTime
        let duration = std::time::Duration::from_secs_f64(seconds);
        Ok(DeltaTime::new(duration))
    } else {
        Err(ConfigError::ValidationError(
            "DeltaTime resource requires a 'delta_seconds' field as a number".to_string(),
        ))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const TEST_CONFIG_JSON: &str = r#"
    {
      "start_state": {
        "entities": [
          {"id": 0, "components": {"position": {"x": 0.0, "y": 0.0}}},
          {"id": 1, "components": {"position": {"x": 10.0, "y": 5.0}}}
        ]
      },
      "transport": {
        "serializer": {"type": "binary"},
        "sender": {"type": "file", "options": {"path": "/tmp/output.bin"}}
      },
      "simulation": {"threads": 1, "fps": 60}
    }
    "#;

     const INVALID_CONFIG_JSON: &str = r#"
    {
      "start_state": {
        "entities": []
      },
      "transport": {
        "serializer": {"type": "binary"},
        "sender": {"type": "file"}
      },
      "simulation": {"threads": 1, "fps": 0}
    }
    "#;


    #[test]
    fn test_load_valid_config() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", TEST_CONFIG_JSON).unwrap();

        let config = load_config(file.path()).unwrap();

        assert_eq!(config.start_state.entities.len(), 2);
        assert_eq!(config.start_state.entities[0].id, 0);
        assert!(config.start_state.entities[0].components.contains_key("position"));
        assert_eq!(config.transport.serializer.type_, "binary");
        assert_eq!(config.transport.sender.type_, "file");
        assert_eq!(config.transport.sender.options.as_ref().unwrap().get("path").unwrap().as_str().unwrap(), "/tmp/output.bin");
        assert_eq!(config.simulation.threads, 1);
        assert_eq!(config.simulation.fps, 60);
    }

     #[test]
    fn test_load_invalid_config_value() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", INVALID_CONFIG_JSON).unwrap();
        let result = load_config(file.path());
        assert!(result.is_err());
        match result.err().unwrap() {
             ConfigError::ValidationError(msg) => assert!(msg.contains("FPS cannot be zero")), // Or file sender path missing
             _ => panic!("Expected ValidationError"),
         }
    }

    #[test]
    fn test_parse_position() {
        let json_val: serde_json::Value = serde_json::from_str(r#"{"x": 1.2, "y": -3.4}"#).unwrap();
        let position = parse_position_component(&json_val).unwrap();
        assert_eq!(position.x, 1.2);
        assert_eq!(position.y, -3.4);
    }

     #[test]
    fn test_parse_invalid_position() {
        let json_val: serde_json::Value = serde_json::from_str(r#"{"x": 1.2}"#).unwrap(); // Missing y
        let result = parse_position_component(&json_val);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_delta_time() {
        let json_val: serde_json::Value = serde_json::from_str(r#"{"delta_seconds": 0.016}"#).unwrap();
        let delta_time = parse_delta_time_resource(&json_val).unwrap();
        assert_eq!(delta_time.delta_seconds, 0.016);
    }

    #[test]
    fn test_parse_invalid_delta_time() {
        let json_val: serde_json::Value = serde_json::from_str(r#"{"wrong_field": 0.016}"#).unwrap();
        let result = parse_delta_time_resource(&json_val);
        assert!(result.is_err());
    }
}
