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
// pub mod setup; // Old particle setup
pub mod environment_setup; // New: Spawns nest and food
pub mod ant_setup;         // New: Spawns ants
pub mod state_export;
pub mod transport_integration;
// Add new modules for ant logic:
pub mod ant_logic;
pub mod ant_movement;
pub mod pheromones; // For Phase 2

// Re-export the primary system function from each module for convenient use in schedule setup.
pub use movement::move_particles;
pub use randomization::randomize_velocities; // Keep for now, might remove if ant_movement fully replaces it
pub use boundary::handle_boundaries;
// pub use setup::spawn_particles; // Old particle setup removed
pub use environment_setup::setup_environment_system; // New
pub use ant_setup::spawn_ants_system;             // New
pub use ant_logic::{ant_state_machine_system, update_ant_timers_system}; // New, added timer system
pub use ant_movement::ant_movement_system;        // New
pub use state_export::update_current_simulation_state_resource;
pub use transport_integration::send_simulation_data_system;

// Removed the inline transport_integration module
