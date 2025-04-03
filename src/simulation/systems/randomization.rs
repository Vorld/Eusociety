use bevy_ecs::prelude::*;
use crate::simulation::components::Velocity;
use crate::simulation::resources::{Time, SimulationConfigResource};
use rand::random;

/// System for adding small random changes to particle velocities
pub fn randomize_velocities(
    mut query: Query<&mut Velocity>,
    simulation_config: Res<SimulationConfigResource>,
    time: Res<Time>,
) {
    let factor = simulation_config.0.velocity_randomization_factor;
    for mut velocity in query.iter_mut() {
        // Add small random changes for smooth movement
        velocity.dx += (random::<f32>() - 0.5) * factor * time.delta_seconds;
        velocity.dy += (random::<f32>() - 0.5) * factor * time.delta_seconds;
    }
}