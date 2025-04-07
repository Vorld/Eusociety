//! Contains the Bevy system for applying randomization and damping to particle velocities.

use bevy_ecs::prelude::*;
use crate::simulation::components::Velocity;
use crate::simulation::resources::{Time, SimulationConfigResource};
use rand::thread_rng;
use rand::Rng;

/// Bevy system that applies damping and random adjustments to particle velocities.
///
/// This system simulates effects like drag/friction (damping) and adds small,
/// random fluctuations to velocity, preventing particles from moving in perfectly
/// straight lines indefinitely. It also clamps the velocity to the configured `max_velocity`.
/// Uses parallel iteration (`par_iter_mut`) and thread-local RNG for efficiency.
///
/// # Arguments
///
/// * `query` - A Bevy query to access mutable `Velocity` components of particles.
/// * `simulation_config` - The resource containing simulation parameters like randomization factor, damping factor, and max velocity.
/// * `time` - The `Time` resource providing delta time information.
pub fn randomize_velocities(
    mut query: Query<&mut Velocity>,
    simulation_config: Res<SimulationConfigResource>,
    time: Res<Time>,
) {
    let randomization_factor = simulation_config.0.velocity_randomization_factor;
    let damping_factor = simulation_config.0.velocity_damping_factor;
    let delta_seconds = time.delta_seconds;
    // Pre-calculate max_velocity_squared outside the loop
    let max_velocity = simulation_config.0.max_velocity;
    let max_velocity_squared = max_velocity * max_velocity; 
    
    // Using par_for_each for parallel iteration
    query.par_iter_mut().for_each(|mut velocity| {
        let mut rng = thread_rng(); // Thread-local RNG for parallelism

        // Apply damping to reduce momentum influence
        velocity.dx *= damping_factor;
        velocity.dy *= damping_factor;

        // Add small random changes for smooth movement
        velocity.dx += (rng.gen::<f32>() - 0.5) * randomization_factor * delta_seconds;
        velocity.dy += (rng.gen::<f32>() - 0.5) * randomization_factor * delta_seconds;

        // --- Optimization: Use squared comparison to avoid sqrt ---
        let speed_squared = velocity.dx.powi(2) + velocity.dy.powi(2);

        // Clamp the velocity only if necessary
        if speed_squared > max_velocity_squared {
            // Only calculate sqrt when needed
            let speed = speed_squared.sqrt(); 
            let scale = max_velocity / speed;
            velocity.dx *= scale;
            velocity.dy *= scale;
        }
        // --- End Optimization ---

    });
}
