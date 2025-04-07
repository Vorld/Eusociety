//! System responsible for updating ant velocities based on their state and environment.
//! Currently implements a basic random walk. Pheromone influence will be added later.

use bevy_ecs::prelude::*;
use crate::simulation::components::{Ant, AntState, Velocity};
use crate::simulation::resources::{SimulationConfigResource, Time};
use rand::{thread_rng, Rng};

/// System that adjusts ant velocities to simulate wandering behavior.
/// Will be expanded later to incorporate pheromone following.
pub fn ant_movement_system(
    mut query: Query<(&AntState, &mut Velocity), With<Ant>>,
    config: Res<SimulationConfigResource>,
    time: Res<Time>,
) {
    let mut rng = thread_rng();
    let max_velocity = config.0.max_velocity;
    let randomization_factor = config.0.velocity_randomization_factor;
    let damping_factor = config.0.velocity_damping_factor;
    let delta_seconds = time.delta_seconds;
    
    // Prefix unused ant_state with _
    for (_ant_state, mut velocity) in query.iter_mut() {
        // Apply damping
        velocity.dx *= damping_factor;
        velocity.dy *= damping_factor;

        // Apply random walk adjustment
        // Uses configuration-based randomization with no hardcoded multipliers
        let change_x = rng.gen_range(-0.5..=0.5) * randomization_factor * delta_seconds;
        let change_y = rng.gen_range(-0.5..=0.5) * randomization_factor * delta_seconds;

        velocity.dx += change_x;
        velocity.dy += change_y;

        // Clamp velocity to max_velocity
        let speed_sq = velocity.dx * velocity.dx + velocity.dy * velocity.dy;
        if speed_sq > max_velocity * max_velocity {
            let speed = speed_sq.sqrt();
            velocity.dx = (velocity.dx / speed) * max_velocity;
            velocity.dy = (velocity.dy / speed) * max_velocity;
        }

        // TODO (Phase 2): Based on ant_state, query nearby pheromones
        // TODO (Phase 2): Calculate pheromone influence vector
        // TODO (Phase 2): Blend influence vector with current velocity adjustments
    }
}
