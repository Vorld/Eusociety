//! Contains the Bevy system responsible for sending the simulation state via the transport layer.

use bevy_ecs::prelude::*;
use tracing::error;
use crate::simulation::resources::CurrentSimulationState;
use crate::transport::TransportController;

/// Bevy system that takes the `CurrentSimulationState` resource and sends it
/// using the `TransportController` resource.
///
/// This system should run *after* `update_current_simulation_state_resource`
/// to ensure it sends the latest state for the frame. The actual serialization
/// and sending logic (including handling different senders like WebSocket or File)
/// is encapsulated within the `TransportController`.
///
/// # Arguments
///
/// * `state` - Read-only access to the `CurrentSimulationState` resource containing the data to send.
/// * `controller` - Mutable access to the `TransportController` resource, used to perform the send operation.
pub fn send_simulation_data_system(
    state: Res<CurrentSimulationState>,
    mut controller: ResMut<TransportController>, 
) {
    // Send the state by reference, avoiding the clone
    if let Err(e) = controller.send_simulation_state(&state.0) {
        error!("Failed to send simulation state via Bevy system: {}", e);
        // Consider adding more robust error handling if needed
    }
}
