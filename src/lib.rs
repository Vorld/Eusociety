//! # Eusociety Simulation Library
//!
//! This library provides the core logic for a particle simulation, including:
//! - Configuration loading and validation (`config` module).
//! - Particle simulation using the Bevy ECS framework (`simulation` module).
//! - Data serialization and transport (e.g., via WebSockets or file output) (`transport` module).
//!
//! The simulation models particles moving within defined boundaries, with configurable
//! behaviors and transport mechanisms.

pub mod config;     // Configuration handling
pub mod simulation;  // Particle simulation components, resources and systems
pub mod transport;   // Data serialization and transport

// Re-export commonly used items
pub mod prelude {
    // Removed SerializerType, added SerializerConfig
    pub use crate::config::{Config, SimulationConfig, TransportConfig, BoundaryBehavior, SerializerConfig};
    pub use crate::simulation::components::{Position, Velocity, ParticleId};
    pub use crate::transport::{SerializationError, TransportError, TransportController};
}
