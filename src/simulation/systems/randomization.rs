use bevy_ecs::prelude::*;
use crate::simulation::components::Velocity;
use crate::simulation::resources::{Time, SimulationConfigResource};
use rand::thread_rng;
use rand::Rng;

/// System for adding small random changes to particle velocities
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
