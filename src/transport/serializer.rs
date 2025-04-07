//! Defines traits and implementations for serializing simulation data into byte streams.
//!
//! This module provides:
//! - The `Serializer` trait for defining different serialization methods.
//! - Concrete implementations: `JsonSerializer`, `BinarySerializer`, `NullSerializer`.
//! - An `OptimizedBinarySerializer` that can incorporate delta compression and parallel processing.
//! - Helper traits (`SerializerClone`, `SerializeObject`) and error types (`SerializationError`).

use serde::Serialize;
use thiserror::Error;
use rayon::prelude::*;

// Import DeltaCompressor from the delta_compression module
use super::delta_compression::DeltaCompressor; 

/// Error types that can occur during serialization.
#[derive(Error, Debug)]
pub enum SerializationError {
    /// Error during JSON serialization (from `serde_json`).
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// Error during binary serialization (from `bincode`).
    #[error("Binary serialization error: {0}")]
    BinaryError(#[from] bincode::Error),
    /// Custom error during parallel serialization logic.
    #[error("Parallel serialization error: {0}")]
    ParallelError(String),
}

// --- Core Serializer Traits ---

// Note: Original SerializerType enum removed as SerializerConfig handles type definition.
/// Base trait for serializers.
///
/// Defines the core function `serialize_to_bytes` which takes a trait object
/// implementing `SerializeObject` and returns its byte representation.
/// Requires `Send + Sync + SerializerClone` for thread safety and clonability
/// when used as a trait object (`Box<dyn Serializer>`).
pub trait Serializer: Send + Sync + SerializerClone {
    /// Serializes the given data object into a byte vector.
    ///
    /// # Arguments
    ///
    /// * `data` - A trait object implementing `SerializeObject`.
    ///
    /// # Errors
    ///
    /// Returns `SerializationError` if the underlying serialization process fails.
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError>;
}

/// Enables cloning of `Box<dyn Serializer>`.
impl Clone for Box<dyn Serializer> {
    fn clone(&self) -> Self {
        self.clone_serializer() // Delegates to the object-safe clone method
    }
}

/// A serializer that performs no operation and returns an empty byte vector.
/// Useful for disabling serialization/transport entirely via configuration.
#[derive(Clone)]
pub struct NullSerializer;

impl Serializer for NullSerializer {
    /// Returns `Ok(Vec::new())` immediately.
    fn serialize_to_bytes(&self, _data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        Ok(Vec::new())
    }
}

/// Helper trait providing an object-safe cloning method for `Serializer`.
/// This is necessary to allow `Box<dyn Serializer>` to be cloneable.
pub trait SerializerClone {
    /// Creates a boxed clone of the `Serializer`.
    fn clone_serializer(&self) -> Box<dyn Serializer>;
}

// Implement `SerializerClone` for each concrete serializer type.
impl SerializerClone for NullSerializer {
    fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
    }
}

/// Trait implemented by data structures that can be serialized to JSON or binary formats.
/// This allows the `Serializer` trait to work with different concrete types via dynamic dispatch.
pub trait SerializeObject {
    /// Serializes the object to a JSON byte vector.
    fn to_json(&self) -> Result<Vec<u8>, SerializationError>;
    /// Serializes the object to a binary byte vector using `bincode`.
    fn to_binary(&self) -> Result<Vec<u8>, SerializationError>;
}

/// Blanket implementation of `SerializeObject` for any type `T` that implements `serde::Serialize`.
impl<T: Serialize + ?Sized> SerializeObject for T {
    /// Uses `serde_json::to_vec` for serialization.
    fn to_json(&self) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(self).map_err(SerializationError::JsonError)
    }
    /// Uses `bincode::serialize` for serialization.
    fn to_binary(&self) -> Result<Vec<u8>, SerializationError> {
        bincode::serialize(self).map_err(SerializationError::BinaryError)
    }
}

// --- Concrete Serializer Implementations ---

/// Serializer implementation using `serde_json`.
#[derive(Clone)]
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    /// Serializes the data object to JSON bytes using its `to_json` method.
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        data.to_json()
    }
}

impl SerializerClone for JsonSerializer {
    fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
    }
}

/// Serializer implementation using `bincode`.
#[derive(Clone)]
pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    /// Serializes the data object to binary bytes using its `to_binary` method.
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        data.to_binary()
    }
}

impl SerializerClone for BinarySerializer {
    fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
    }
}

// --- Delta Compression Logic Moved to delta_compression.rs ---

/// An optimized binary serializer primarily intended for `SimulationState`.
///
/// This serializer can optionally:
/// - Use a `DeltaCompressor` to filter out unchanged particle data before serialization.
/// - Employ parallel processing (`rayon`) for serializing large numbers of particles
///   while maintaining compatibility with the expected `bincode` format.
#[derive(Clone)]
pub struct OptimizedBinarySerializer {
    /// Optional delta compressor instance. If `Some`, `filter_state` is called before serialization.
    delta_compressor: Option<DeltaCompressor>,
    /// Flag to enable/disable parallel serialization.
    use_parallel: bool,
    /// Minimum number of particles required to trigger parallel serialization logic.
    parallel_threshold: usize,
    /// Number of threads hint for Rayon (0 = automatic).
    thread_count: usize,
}

impl OptimizedBinarySerializer {
    /// Creates a new `OptimizedBinarySerializer`.
    ///
    /// # Arguments
    ///
    /// * `delta_threshold` - If `Some(threshold)`, enables delta compression with the given
    ///   movement threshold. If `None`, delta compression is disabled.
    pub fn new(delta_threshold: Option<f32>) -> Self {
        // Create delta compressor if a threshold is provided
        let delta_compressor = delta_threshold.map(DeltaCompressor::new);
            
        Self { 
            delta_compressor,
            use_parallel: true, // Default to enabled
            parallel_threshold: 50000, // Default threshold
            thread_count: 0,           // Default thread count (auto)
        }
    }
    
    /// Serializes a `SimulationState` object, applying optimizations.
    ///
    /// This method first applies delta compression (if enabled), then chooses between
    /// sequential `bincode` serialization or a custom parallel serialization implementation
    /// based on the number of particles and the `use_parallel` flag.
    ///
    /// # Arguments
    ///
    /// * `state` - The `SimulationState` to serialize.
    ///
    /// # Errors
    ///
    /// Returns `SerializationError` if any serialization step fails.
    pub fn serialize_state(&mut self, state: &super::SimulationState) -> Result<Vec<u8>, SerializationError> {
        // 1. Apply delta compression if enabled
        let final_state = if let Some(compressor) = &mut self.delta_compressor {
            compressor.filter_state(state)
        } else {
            state.clone() // Clone if no delta compression needed
        };
        
        // 2. Choose serialization strategy based on particle count and config
        if self.use_parallel && final_state.particles.len() >= self.parallel_threshold {
            // Use parallel serialization for large states
            self.serialize_state_parallel_compatible(&final_state)
        } else {
            // Use standard sequential bincode serialization for smaller states
            bincode::serialize(&final_state)
                .map_err(SerializationError::BinaryError)
        }
    }

    /// Internal helper for parallel serialization of `SimulationState`.
    ///
    /// Serializes the header (frame, timestamp, particle count) sequentially,
    /// then serializes particle data in parallel chunks using Rayon, and finally
    /// concatenates the results. Designed to produce output compatible with
    /// standard `bincode` deserialization on the receiving end.
    ///
    /// # Arguments
    ///
    /// * `state` - The `SimulationState` (potentially delta-compressed) to serialize.
    ///
    /// # Errors
    ///
    /// Returns `SerializationError` if header or particle chunk serialization fails.
    fn serialize_state_parallel_compatible(&self, state: &super::SimulationState) -> Result<Vec<u8>, SerializationError> {
        // Estimate buffer size (can be approximate)
        let particle_size = std::mem::size_of::<u32>() + std::mem::size_of::<f32>() * 2; // id, x, y
        let header_size = std::mem::size_of::<u64>() * 2 + std::mem::size_of::<f64>(); // frame, count, timestamp
        let estimated_capacity = header_size + state.particles.len() * particle_size;

        // --- Parallel Serialization Steps ---

        // 1. Serialize header (frame, timestamp, particle count) sequentially
        let mut final_buffer = Vec::with_capacity(estimated_capacity);
        { // Scope to borrow final_buffer mutably
            // Frame
            final_buffer.extend_from_slice(&bincode::serialize(&state.frame)?);
            // Timestamp
            final_buffer.extend_from_slice(&bincode::serialize(&state.timestamp)?);
            // Particle count (as u64 for bincode Vec length prefix)
            final_buffer.extend_from_slice(&bincode::serialize(&(state.particles.len() as u64))?);
        }

        // 2. Serialize particle data in parallel chunks
        // Build the Rayon thread pool, configuring the number of threads if specified
        let pool = {
            let builder = rayon::ThreadPoolBuilder::new();
            if self.thread_count > 0 {
                builder.num_threads(self.thread_count) // Consumes builder, returns new one
            } else {
                builder // Use the original builder
            }
            .build() // Build from the final builder instance
            .map_err(|e| SerializationError::ParallelError(format!("Failed to build Rayon pool: {}", e)))?
        };

        // Install the pool context for parallel iteration
        let particle_chunks: Result<Vec<Vec<u8>>, SerializationError> = pool.install(|| {
            state.particles
                .par_chunks(self.parallel_threshold.max(1)) // Ensure chunk size is at least 1
                .map(|particle_chunk| {
                    // Serialize each chunk into its own buffer
                    let mut chunk_buffer = Vec::with_capacity(particle_chunk.len() * particle_size);
                    for particle in particle_chunk {
                        // Serialize fields individually for compatibility
                        chunk_buffer.extend_from_slice(&bincode::serialize(&particle.id)?);
                        chunk_buffer.extend_from_slice(&bincode::serialize(&particle.x)?);
                        chunk_buffer.extend_from_slice(&bincode::serialize(&particle.y)?);
                    }
                    Ok(chunk_buffer)
                })
                .collect() // Collect results within the Rayon pool context
        });

        // Check for errors during parallel processing
        let collected_chunks = particle_chunks?;

        // 3. Concatenate header and parallel chunks
        for chunk in collected_chunks {
            final_buffer.extend_from_slice(&chunk);
        }

        Ok(final_buffer)
        // --- End Parallel Serialization Steps ---
    }
    
    /// Returns `true` if delta compression is configured for this serializer.
    pub fn has_delta_compression(&self) -> bool {
        self.delta_compressor.is_some()
    }
    
    /// Enables or disables parallel serialization.
    pub fn set_parallel(&mut self, enabled: bool) -> &mut Self {
        self.use_parallel = enabled;
        self
    }
    
    /// Sets the minimum number of particles required to trigger parallel serialization.
    pub fn set_parallel_threshold(&mut self, threshold: usize) -> &mut Self {
        self.parallel_threshold = threshold;
        self
    }
    
    /// Sets the number of threads for Rayon to use (0 means automatic).
    pub fn set_thread_count(&mut self, count: usize) -> &mut Self {
        self.thread_count = count;
        self
    }
    
    /// Gets the current parallel serialization threshold.
    pub fn parallel_threshold(&self) -> usize {
        self.parallel_threshold
    }
    
    /// Gets the current thread count setting (0 means automatic).
    pub fn thread_count(&self) -> usize {
        self.thread_count
    }
    
    /// Returns `true` if parallel serialization is enabled.
    pub fn is_parallel(&self) -> bool {
        self.use_parallel
    }
}

impl Serializer for OptimizedBinarySerializer {
    /// Serializes arbitrary `SerializeObject` data using standard `bincode`.
    /// Note: This does *not* use the delta compression or parallel optimizations,
    /// as those are specific to the `SimulationState` structure in `serialize_state`.
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        data.to_binary()
    }
}

impl SerializerClone for OptimizedBinarySerializer {
     fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
    }
}
