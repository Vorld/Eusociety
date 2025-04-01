use serde::Deserialize;
use std::path::Path;
use std::{fs, io};

// --- Error Type ---
#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Parse(serde_json::Error),
    Validation(String),
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self { ConfigError::Io(err) }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self { ConfigError::Parse(err) }
}

// --- Enums for Choices ---
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SerializerType {
    Json,
    Binary,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SenderType {
    Stdio,
    WebSocket,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BehaviorType {
    Random,
    Flocking,
    #[serde(other)]
    Other,
}

// Add Default implementation for BehaviorType
impl Default for BehaviorType {
    fn default() -> Self {
        BehaviorType::Random  // Default to random movement behavior
    }
}

// --- Configuration Sections ---

#[derive(Deserialize, Debug, Clone)]
pub struct WorldSettings {
    pub width: f32,
    pub height: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FlockingSettings {
    #[serde(default = "default_perception_radius")]
    pub perception_radius: f32,
    #[serde(default = "default_separation_weight")]
    pub separation_weight: f32,
    #[serde(default = "default_alignment_weight")]
    pub alignment_weight: f32,
    #[serde(default = "default_cohesion_weight")]
    pub cohesion_weight: f32,
}

// Default values for flocking behavior
fn default_perception_radius() -> f32 { 10.0 }
fn default_separation_weight() -> f32 { 0.5 }
fn default_alignment_weight() -> f32 { 0.3 }
fn default_cohesion_weight() -> f32 { 0.2 }

#[derive(Deserialize, Debug, Clone)]
pub struct InitialEntityConfig {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub count: u32,
    #[serde(default)]
    pub behavior: BehaviorType,
    pub settings: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SpatialGridConfig {
    #[serde(default = "default_cell_size")]
    pub cell_size: f32,
}

fn default_cell_size() -> f32 { 5.0 }

#[derive(Deserialize, Debug, Clone)]
pub struct SerializerConfig {
    #[serde(rename = "type")]
    pub serializer_type: SerializerType,
    pub options: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SenderConfig {
    #[serde(rename = "type")]
    pub sender_type: SenderType,
    pub options: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TransportConfig {
    pub serializer: SerializerConfig,
    pub sender: SenderConfig,
}

// --- Top-Level Config Struct ---

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub framerate: u32,
    pub world_settings: WorldSettings,
    pub initial_state: Vec<InitialEntityConfig>,
    #[serde(default)]
    pub spatial_grid: SpatialGridConfig,
    pub transport: TransportConfig,
}

// Default implementation for SpatialGridConfig
impl Default for SpatialGridConfig {
    fn default() -> Self {
        Self {
            cell_size: default_cell_size(),
        }
    }
}

// --- WebSocket Configuration ---
#[derive(Deserialize, Debug, Clone)]
pub struct WebSocketOptions {
    #[serde(default = "default_ws_host")]
    pub host: String,
    #[serde(default = "default_ws_port")]
    pub port: u16,
}

fn default_ws_host() -> String { "127.0.0.1".to_string() }
fn default_ws_port() -> u16 { 8080 }

impl Default for WebSocketOptions {
    fn default() -> Self {
        Self {
            host: default_ws_host(),
            port: default_ws_port(),
        }
    }
}

// --- Helper Methods ---

impl InitialEntityConfig {
    /// Parse flocking settings if present
    pub fn get_flocking_settings(&self) -> Option<FlockingSettings> {
        if self.behavior == BehaviorType::Flocking {
            if let Some(value) = &self.settings {
                return serde_json::from_value(value.clone()).ok();
            }
            // Return default settings if no specific settings provided
            return Some(FlockingSettings {
                perception_radius: default_perception_radius(),
                separation_weight: default_separation_weight(),
                alignment_weight: default_alignment_weight(),
                cohesion_weight: default_cohesion_weight(),
            });
        }
        None
    }
}

// --- Loading Function ---

pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;

    // Basic Validation
    if config.framerate == 0 {
        return Err(ConfigError::Validation("Framerate cannot be zero.".to_string()));
    }
    
    // M1 validation - remove these restrictions in future versions
    if config.initial_state.iter().any(|e| e.entity_type != "walker") {
         return Err(ConfigError::Validation("Only 'walker' entity type is supported in M1.".to_string()));
    }
    
    // Allow both JSON and binary serializers now
    if config.transport.serializer.serializer_type != SerializerType::Json && 
       config.transport.serializer.serializer_type != SerializerType::Binary {
         return Err(ConfigError::Validation("Only 'json' and 'binary' serializers are supported.".to_string()));
    }
    
    // Allow both stdio and websocket senders
    if config.transport.sender.sender_type != SenderType::Stdio && 
       config.transport.sender.sender_type != SenderType::WebSocket {
         return Err(ConfigError::Validation("Only 'stdio' and 'websocket' senders are supported.".to_string()));
    }

    Ok(config)
}

// Helper methods for extracting options
impl SenderConfig {
    pub fn get_websocket_options(&self) -> WebSocketOptions {
        if let Some(value) = &self.options {
            if let Ok(options) = serde_json::from_value(value.clone()) {
                return options;
            }
        }
        WebSocketOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn load_valid_config_m1() {
        let content = r#"{
          "framerate": 60,
          "world_settings": { "width": 100.0, "height": 80.0 },
          "initial_state": [ { "type": "walker", "count": 50 } ],
          "transport": {
            "serializer": { "type": "json", "options": null },
            "sender": { "type": "stdio", "options": null }
          }
        }"#;
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        let config = load_config(file.path()).unwrap();

        assert_eq!(config.framerate, 60);
        assert_eq!(config.world_settings.width, 100.0);
        assert_eq!(config.initial_state.len(), 1);
        assert_eq!(config.initial_state[0].entity_type, "walker");
        assert_eq!(config.initial_state[0].count, 50);
        assert_eq!(config.transport.serializer.serializer_type, SerializerType::Json);
        assert_eq!(config.transport.sender.sender_type, SenderType::Stdio);
    }

     #[test]
    fn load_invalid_framerate() {
        let content = r#"{
          "framerate": 0,
          "world_settings": { "width": 100.0, "height": 80.0 },
          "initial_state": [ { "type": "walker", "count": 50 } ],
          "transport": {
            "serializer": { "type": "json" },
            "sender": { "type": "stdio" }
          }
        }"#;
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        let result = load_config(file.path());
        assert!(matches!(result, Err(ConfigError::Validation(_))));
    }

    // Add more tests for other validation rules
}
