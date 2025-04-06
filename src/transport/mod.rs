mod serializer;
mod sender;

use bevy_ecs::prelude::Resource; // Added import
use serde::Serialize;
use std::time::Instant;
use tracing::{debug, info, error}; // Added error import for potential use

// Re-export types
pub use self::serializer::{
    Serializer, SerializationError, JsonSerializer, BinarySerializer, NullSerializer,
    SerializeObject, OptimizedBinarySerializer, DeltaCompressor
};
pub use self::sender::{Sender, TransportError, FileSender, WebSocketSender, NullSender};
use crate::config::{SenderConfig, TransportConfig, SerializerConfig};

/// Particle state for serialization
#[derive(Serialize, Clone, Debug)] // Added Debug
pub struct ParticleState {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

/// Complete simulation state for serialization
#[derive(Serialize, Clone, Debug, Default)] // Added Debug, Default
pub struct SimulationState {
    pub frame: u64,
    pub timestamp: f64,
    pub particles: Vec<ParticleState>,
}

/// Controller for handling serialization and transport of simulation data
#[derive(Resource, Clone)] // Added Resource derive
pub struct TransportController {
    serializer: Box<dyn Serializer>,
    sender: Box<dyn Sender>,
    optimized_serializer: Option<OptimizedBinarySerializer>,
    update_frequency: Option<u32>,
    current_frame: u32,
    // Performance metrics
    last_serialization_time_ms: f64,
    last_send_time_ms: f64,
    last_data_size_bytes: usize,
}

impl TransportController {
    /// Create a new transport controller with the provided serializer and sender
    pub fn new(
        serializer: Box<dyn Serializer>,
        sender: Box<dyn Sender>,
    ) -> Self {
        Self {
            serializer,
            sender,
            optimized_serializer: None,
            update_frequency: None,
            current_frame: 0,
            last_serialization_time_ms: 0.0,
            last_send_time_ms: 0.0,
            last_data_size_bytes: 0,
        }
    }

    /// Create a transport controller from configuration
    pub fn from_config(config: &TransportConfig) -> Result<Self, TransportError> {
        // Create serializer based on the SerializerConfig enum
        let serializer: Box<dyn Serializer> = match &config.serializer {
            SerializerConfig::Json(_) => Box::new(JsonSerializer),
            SerializerConfig::Binary(_) => Box::new(BinarySerializer),
            SerializerConfig::Null(_) => Box::new(NullSerializer),
        };

        let mut _update_frequency: Option<u32> = None; // Prefixed with _
        let mut _optimized_serializer: Option<OptimizedBinarySerializer> = None; // Prefixed with _

        // These variables will be properly assigned within the match below
        let update_frequency: Option<u32>;
        let optimized_serializer: Option<OptimizedBinarySerializer>;

        // Create sender based on the SenderConfig enum
        let sender: Box<dyn Sender> = match &config.sender {
            SenderConfig::File(file_config) => {
                update_frequency = Some(file_config.output_frequency);
                optimized_serializer = None;
                Box::new(FileSender::new(&file_config.output_path)?)
            },
            SenderConfig::WebSocket(ws_config) => {
                update_frequency = Some(ws_config.update_frequency);
                
                // Create optimized binary serializer with delta compression if enabled
                let threshold = if config.delta_compression == Some(true) {
                    // Use the configured threshold or default to 0.1 if not specified
                    let threshold_value = config.delta_threshold.unwrap_or(0.1);
                    info!("Delta compression enabled with threshold {}", threshold_value);
                    Some(threshold_value)
                } else {
                    info!("Delta compression disabled");
                    None
                };
                
                // Create the optimized serializer
                let mut opt_serializer = OptimizedBinarySerializer::new(threshold);
                
                // Configure parallel serialization if specified
                if let Some(parallel_config) = &config.parallel_serialization {
                    opt_serializer.set_parallel(parallel_config.enabled);
                    
                    if let Some(threshold) = parallel_config.threshold {
                        opt_serializer.set_parallel_threshold(threshold);
                    }
                    
                    if let Some(thread_count) = parallel_config.thread_count {
                        opt_serializer.set_thread_count(thread_count);
                    }
                    
                    // Log the parallel serialization configuration
                    if parallel_config.enabled {
                        info!(
                            threshold = opt_serializer.parallel_threshold(),
                            threads = if opt_serializer.thread_count() == 0 { 
                                format!("auto ({})", rayon::current_num_threads())
                            } else {
                                opt_serializer.thread_count().to_string()
                            },
                            "Parallel serialization enabled"
                        );
                    } else {
                        info!("Parallel serialization disabled");
                    }
                } else {
                    // Default to enabled for WebSocket with large particle counts
                    info!("Parallel serialization enabled with default settings");
                }
                
                optimized_serializer = Some(opt_serializer);
                Box::new(WebSocketSender::new(&ws_config.websocket_address)?)
            },
            SenderConfig::Null(_) => {
                update_frequency = None;
                optimized_serializer = None;
                Box::new(NullSender)
            }
        };

        // Create the controller instance
        let mut controller = Self::new(serializer, sender);
        controller.optimized_serializer = optimized_serializer;
        controller.update_frequency = update_frequency;

        Ok(controller)
    }

    /// Serialize and send simulation state (Generic version, might be less used now)
    pub fn send_state<T: SerializeObject + Serialize>(&mut self, state: &T) -> Result<(), TransportError> {
        let serialization_start = Instant::now();
        
        // Serialize data
        let data = self.serializer.serialize_to_bytes(state)
            .map_err(TransportError::SerializationError)?;
            
        let serialization_time = serialization_start.elapsed();
        self.last_serialization_time_ms = serialization_time.as_secs_f64() * 1000.0;
        self.last_data_size_bytes = data.len();

        let send_start = Instant::now();
        
        // Send data through sender
        self.sender.send(&data)?;
        
        let send_time = send_start.elapsed();
        self.last_send_time_ms = send_time.as_secs_f64() * 1000.0;

        debug!(
            serialization_ms = self.last_serialization_time_ms,
            send_ms = self.last_send_time_ms,
            data_size_bytes = self.last_data_size_bytes,
            "Transport metrics"
        );

        Ok(())
    }

    /// Serialize and send simulation state with optimized serializer if available
    pub fn send_simulation_state(&mut self, state: &SimulationState) -> Result<(), TransportError> {
        // Increment frame counter
        self.current_frame += 1;

        // Check if we should send this frame based on update frequency
        if let Some(freq) = self.update_frequency {
            if freq > 0 && self.current_frame % freq != 0 {
                return Ok(());
            }
        }

        let serialization_start = Instant::now();
        
        // Use optimized serializer if available (typically for WebSocket)
        let data = if let Some(serializer) = &mut self.optimized_serializer {
            // Count particles before potentially filtering them
            let original_particle_count = state.particles.len();
            
            // Serialize with potentially filtering out unchanged particles
            let result = serializer.serialize_state(state)
                .map_err(TransportError::SerializationError)?;
                
            // Log detailed info if using delta compression
            if serializer.has_delta_compression() {
                debug!(
                    original_particles = original_particle_count,
                    serialized_size_bytes = result.len(),
                    "Delta compression metrics"
                );
            }
            
            result
        } else {
            // Fallback to the standard serializer
            self.serializer.serialize_to_bytes(state)
                .map_err(TransportError::SerializationError)?
        };
        
        let serialization_time = serialization_start.elapsed();
        self.last_serialization_time_ms = serialization_time.as_secs_f64() * 1000.0;
        self.last_data_size_bytes = data.len();

        // Start timing the send operation
        let send_start = Instant::now();
        
        // Send data
        self.sender.send(&data)?;
        
        let send_time = send_start.elapsed();
        self.last_send_time_ms = send_time.as_secs_f64() * 1000.0;

        // Detailed metrics
        info!(
            frame = self.current_frame,
            particles = state.particles.len(),
            serialization_ms = self.last_serialization_time_ms,
            send_ms = self.last_send_time_ms,
            data_size_mb = (self.last_data_size_bytes as f64 / 1_048_576.0),
            "Transport performance"
        );

        Ok(())
    }

    /// Flush the sender to ensure data is written
    pub fn flush(&self) -> Result<(), TransportError> {
        self.sender.flush()
    }

    /// Get a reference to the WebSocket sender if available by checking the sender's type
    pub fn get_websocket_sender(&self) -> Option<&WebSocketSender> {
        self.sender.as_websocket_sender()
    }
    
    /// Get the last measured serialization time in milliseconds
    pub fn last_serialization_time_ms(&self) -> f64 {
        self.last_serialization_time_ms
    }
    
    /// Get the last measured send time in milliseconds
    pub fn last_send_time_ms(&self) -> f64 {
        self.last_send_time_ms
    }
    
    /// Get the last measured data size in bytes
    pub fn last_data_size_bytes(&self) -> usize {
        self.last_data_size_bytes
    }
}
