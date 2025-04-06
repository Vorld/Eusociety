use bevy_ecs::prelude::*;
use tracing::error;

use crate::simulation::resources::CurrentSimulationState;
use crate::transport::TransportController;

pub mod movement;
pub mod randomization;
pub mod boundary;
pub mod setup;
pub mod state_export; // Added state_export

// Re-export system functions for easier access
pub use movement::move_particles;
pub use randomization::randomize_velocities;
pub use boundary::handle_boundaries;
pub use setup::spawn_particles;
// Removed: pub use transport::{extract_and_send, flush_transport, SimulationTimer, SimulationTransport};
pub use state_export::update_current_simulation_state_resource; // Added state_export system
pub use self::transport_integration::send_simulation_data_system; // Export the new system

// --- New module for transport system ---
mod transport_integration {
    use bevy_ecs::prelude::*;
    use tracing::error;
    use crate::simulation::resources::CurrentSimulationState;
    use crate::transport::TransportController;

    /// System to send the current simulation state using the TransportController resource.
    /// This should run after `update_current_simulation_state_resource`.
    pub fn send_simulation_data_system(
        state: Res<CurrentSimulationState>,
        mut controller: ResMut<TransportController>, // Use ResMut to access the controller
    ) {
        // Send the state by reference, avoiding the clone
        if let Err(e) = controller.send_simulation_state(&state.0) {
            error!("Failed to send simulation state via Bevy system: {}", e);
            // Consider adding more robust error handling if needed
        }
    }
}
