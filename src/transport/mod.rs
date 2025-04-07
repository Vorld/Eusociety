//! Handles serialization and transport of simulation data.
//!
//! This module defines traits and implementations for:
//! - Serializing simulation state (`SimulationState`, `ParticleState`) into different formats (JSON, Binary).
//! - Sending serialized data via different methods (File, WebSocket).
//! - Optimizations like delta compression and parallel serialization.
//! - The main `TransportController` resource used by Bevy systems to manage transport.

mod serializer;
mod sender;
pub mod delta_compression; 
pub mod websocket; // Declare the new websocket module

use bevy_ecs::prelude::Resource; // Added import
use serde::Serialize;
use std::time::Instant;
use tracing::{debug, info}; // Added error import for potential use

// Re-export types
pub use self::serializer::{
    Serializer, SerializationError, JsonSerializer, BinarySerializer, NullSerializer,
    SerializeObject, OptimizedBinarySerializer
};
// Re-export DeltaCompressor and its metrics from the new module
pub use self::delta_compression::{DeltaCompressor, DeltaCompressionMetrics};
// Re-export WebSocketSender from the new module
pub use self::websocket::WebSocketSender;
// Re-export other senders and traits from the sender module
pub use self::sender::{Sender, TransportError, FileSender, NullSender, SenderClone};
use crate::config::{SenderConfig, TransportConfig, SerializerConfig};

/// Represents the state of a single particle for serialization and transport.
/// Note: Velocity is often excluded to reduce data size if not needed by the receiver.
#[derive(Serialize, Clone, Debug)] 
pub struct ParticleState {
    /// Unique identifier of the particle (cast to u32 for transport).
    pub id: u32,
    /// X-coordinate of the particle's position.
    pub x: f32,
    /// Y-coordinate of the particle's position.
    pub y: f32,
}

/// Represents the complete state of the simulation at a specific frame, ready for serialization.
#[derive(Serialize, Clone, Debug, Default)] 
pub struct SimulationState {
    /// The simulation frame number for this state snapshot.
    pub frame: u64,
    /// The simulation time elapsed when this state was captured.
    pub timestamp: f64,
    /// A list containing the state of all particles in the simulation for this frame.
    pub particles: Vec<ParticleState>,
}

/// Bevy resource that manages the serialization and sending of simulation state.
///
/// This controller holds the configured serializer and sender implementations
/// and provides the main interface (`send_simulation_state`) for Bevy systems
/// to trigger data transport. It also handles optimizations like delta compression
/// and performance logging based on the `TransportConfig`.
#[derive(Resource, Clone)] 
pub struct TransportController {
    /// The primary serializer (e.g., JSON, Binary) used if optimized serializer is not applicable.
    serializer: Box<dyn Serializer>,
    /// The configured sender (e.g., FileSender, WebSocketSender).
    sender: Box<dyn Sender>,
    /// An optional optimized serializer, typically `OptimizedBinarySerializer` used with WebSockets.
    optimized_serializer: Option<OptimizedBinarySerializer>,
    /// How often (in frames) to send simulation state updates (0 = every frame). `None` if not applicable (e.g., NullSender).
    update_frequency: Option<u32>, 
    /// How often (in frames) to log transport performance metrics (0 = every frame, `None` = never).
    log_frequency: Option<u32>,    
    /// Internal counter for the current frame being processed by the controller.
    current_frame: u32,
    // --- Performance Metrics ---
    /// Time taken for the last serialization operation (in milliseconds).
    last_serialization_time_ms: f64,
    /// Time taken for the last send operation (in milliseconds).
    last_send_time_ms: f64,
    /// Size of the data payload in the last send operation (in bytes).
    last_data_size_bytes: usize,
}

impl TransportController {
    /// Creates a basic `TransportController`. Usually `from_config` is preferred.
    ///
    /// # Arguments
    ///
    /// * `serializer` - A boxed `Serializer` trait object.
    /// * `sender` - A boxed `Sender` trait object.
    pub fn new(
        serializer: Box<dyn Serializer>,
        sender: Box<dyn Sender>,
    ) -> Self {
        Self {
            serializer,
            sender,
            optimized_serializer: None,
            update_frequency: None,
            log_frequency: None, // Initialize log_frequency
            current_frame: 0,
            last_serialization_time_ms: 0.0,
            last_send_time_ms: 0.0,
            last_data_size_bytes: 0,
        }
    }

    /// Creates and configures a `TransportController` based on the provided `TransportConfig`.
    ///
    /// This factory method instantiates the appropriate serializer and sender based on the
    /// configuration, sets up optimizations (delta compression, parallel serialization),
    /// and configures logging and update frequencies.
    ///
    /// # Arguments
    ///
    /// * `config` - The transport configuration settings.
    ///
    /// # Errors
    ///
    /// Returns `TransportError` if sender creation fails (e.g., file I/O error, WebSocket bind error).
    pub fn from_config(config: &TransportConfig) -> Result<Self, TransportError> {
        info!("Configuring transport controller...");
        // Determine base serializer
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
        // Create the controller instance
        let mut controller = Self::new(serializer, sender);
        controller.optimized_serializer = optimized_serializer;
        controller.update_frequency = update_frequency;
        controller.log_frequency = config.log_frequency; // Read log_frequency from config

        // Log the configured log frequency
        match config.log_frequency {
            Some(0) => info!("Transport performance logging enabled for every frame."),
            Some(freq) => info!("Transport performance logging enabled every {} frames.", freq),
            None => info!("Transport performance logging disabled."),
        }

        Ok(controller)
    }

    /// Serializes and sends an arbitrary `SerializeObject` using the base serializer.
    ///
    /// This is a generic method and might be less used than `send_simulation_state`,
    /// which handles specific optimizations for `SimulationState`.
    /// It measures and stores serialization/send times and data size.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the object to send, must implement `SerializeObject` and `Serialize`.
    ///
    /// # Arguments
    ///
    /// * `state` - A reference to the object to serialize and send.
    ///
    /// # Errors
    ///
    /// Returns `TransportError` if serialization or sending fails.
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

    /// Serializes and sends the current `SimulationState`.
    ///
    /// This is the primary method used by Bevy systems to transport simulation data.
    /// It handles:
    /// - Incrementing the internal frame counter.
    /// - Checking the `update_frequency` to decide whether to send on this frame.
    /// - Using the `optimized_serializer` (with delta compression and parallelism if configured)
    ///   if available, otherwise falling back to the base `serializer`.
    /// - Measuring and logging performance metrics based on `log_frequency`.
    ///
    /// # Arguments
    ///
    /// * `state` - A reference to the `SimulationState` to send.
    ///
    /// # Errors
    ///
    /// Returns `TransportError` if serialization or sending fails.
    pub fn send_simulation_state(&mut self, state: &SimulationState) -> Result<(), TransportError> {
        // Increment internal frame counter for frequency checks
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

        // Log performance metrics based on log_frequency
        let should_log = match self.log_frequency {
            Some(0) => true, // Log every frame if 0
            Some(freq) => self.current_frame % freq == 0, // Log every freq frames
            None => false, // Never log if None
        };

        if should_log {
            info!(
                frame = self.current_frame,
                particles = state.particles.len(), // <-- Added comma
                serialization_ms = self.last_serialization_time_ms, // <-- Added comma
                send_ms = self.last_send_time_ms,
                data_size_mb = (self.last_data_size_bytes as f64 / 1_048_576.0),
                "Transport performance"
        );
        } // <-- Added missing closing brace here

        Ok(())
    }

    /// Flushes the underlying sender, if necessary.
    ///
    /// This ensures that any buffered data is written to the destination (e.g., for `FileSender`).
    /// It might be a no-op for senders that send immediately (e.g., `WebSocketSender`).
    ///
    /// # Errors
    ///
    /// Returns `TransportError` if flushing fails.
    pub fn flush(&self) -> Result<(), TransportError> {
        self.sender.flush()
    }

    /// Attempts to get a reference to the underlying `WebSocketSender`, if that's the configured sender type.
    ///
    /// Returns `Some(&WebSocketSender)` if the sender is a WebSocket sender, `None` otherwise.
    /// This allows accessing WebSocket-specific methods like `client_count`.
    pub fn get_websocket_sender(&self) -> Option<&WebSocketSender> {
        // The as_websocket_sender method is defined on the Sender trait
        self.sender.as_websocket_sender() 
    }
    
    /// Returns the serialization time recorded for the last `send_state` or `send_simulation_state` call, in milliseconds.
    pub fn last_serialization_time_ms(&self) -> f64 {
        self.last_serialization_time_ms
    }
    
    /// Returns the send time recorded for the last `send_state` or `send_simulation_state` call, in milliseconds.
    pub fn last_send_time_ms(&self) -> f64 {
        self.last_send_time_ms
    }
    
    /// Returns the data size recorded for the last `send_state` or `send_simulation_state` call, in bytes.
    pub fn last_data_size_bytes(&self) -> usize {
        self.last_data_size_bytes
    }
}
