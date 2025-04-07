//! Organizes and re-exports the various Bevy ECS systems used in the simulation.
//!
//! Systems contain the core logic that operates on components and resources
//! each frame (or during startup).

// Removed unused imports: bevy_ecs::prelude::*, tracing::error, CurrentSimulationState, TransportController
// These were likely needed by the inline module that was removed.

// Declare sub-modules for different system categories
pub mod movement;
pub mod randomization;
pub mod boundary;
pub mod setup;
pub mod state_export; 
pub mod transport_integration; 

// Re-export the primary system function from each module for convenient use in schedule setup.
pub use movement::move_particles;
pub use randomization::randomize_velocities;
pub use boundary::handle_boundaries;
pub use setup::spawn_particles;
// Removed: pub use transport::{extract_and_send, flush_transport, SimulationTimer, SimulationTransport};
pub use state_export::update_current_simulation_state_resource; // Added state_export system
pub use transport_integration::send_simulation_data_system; // Export from the new module file

// Removed the inline transport_integration module
