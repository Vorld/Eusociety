//! System responsible for updating ant velocities based on their state and environment.
//! Currently implements a basic random walk. Pheromone influence will be added later.

use bevy_ecs::prelude::*;
use glam::Vec2; // Added for vector math
use crate::simulation::components::{Ant, AntState, Velocity, PheromoneInfluence}; // Added PheromoneInfluence
use crate::simulation::resources::{SimulationConfigResource, Time};
use rand::{thread_rng, Rng};

// TODO: Load from config?
const PHEROMONE_INFLUENCE_WEIGHT: f32 = 25.0; // How strongly pheromones affect direction (adjust this!)

/// System that adjusts ant velocities based on state, random walk, and pheromone influence.
pub fn ant_movement_system(
    mut query: Query<(&AntState, &mut Velocity, &PheromoneInfluence), With<Ant>>, // Added PheromoneInfluence
    config: Res<SimulationConfigResource>,
    time: Res<Time>,
) {
    let mut rng = thread_rng();
    let max_velocity = config.0.max_velocity;
    let randomization_factor = config.0.velocity_randomization_factor;
    let damping_factor = config.0.velocity_damping_factor;
    let delta_seconds = time.delta_seconds;
    
    for (_ant_state, mut velocity, influence) in query.iter_mut() {
        // 1. Apply damping to current velocity
        let current_velocity_vec = Vec2::new(velocity.dx, velocity.dy) * damping_factor; // Removed mut

        // 2. Calculate random walk adjustment vector for this frame
        let random_walk_delta = Vec2::new(
            rng.gen_range(-0.5..=0.5) * randomization_factor,
            rng.gen_range(-0.5..=0.5) * randomization_factor,
        ) * delta_seconds; // Scale by time

        // 3. Get pheromone influence vector (already calculated in pheromone_follow_system)
        //    Scale it by weight and time delta to treat it as an acceleration/force
        let pheromone_accel = influence.vector * PHEROMONE_INFLUENCE_WEIGHT * delta_seconds;

        // 4. Combine influences: current damped velocity + random walk + pheromone acceleration
        let mut final_velocity_vec = current_velocity_vec + random_walk_delta + pheromone_accel;

        // 5. Clamp final velocity to max_velocity
        let speed_sq = final_velocity_vec.length_squared();
        if speed_sq > max_velocity * max_velocity {
            final_velocity_vec = final_velocity_vec.normalize_or_zero() * max_velocity;
        }

        // 6. Update the Velocity component
        velocity.dx = final_velocity_vec.x;
        velocity.dy = final_velocity_vec.y;
    }
}
