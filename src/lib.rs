pub mod config;     // Configuration handling
pub mod simulation;  // Particle simulation components, resources and systems
pub mod transport;   // Data serialization and transport

// Re-export commonly used items
pub mod prelude {
    pub use crate::config::{Config, SimulationConfig, TransportConfig, BoundaryBehavior, SerializerType};
    pub use crate::simulation::components::{Position, Velocity, ParticleId};
    pub use crate::transport::{SerializationError, TransportError, TransportController};
}