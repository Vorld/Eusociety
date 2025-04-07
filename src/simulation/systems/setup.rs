//! Contains the Bevy startup system for initializing the simulation state, primarily spawning particles.

use bevy_ecs::prelude::*;
use rand::Rng; // Import only Rng trait, `random` is unused
use tracing::info;
use crate::simulation::components::{Position, Velocity, ParticleId};
use crate::simulation::resources::SimulationConfigResource;

/// Bevy startup system responsible for spawning the initial set of particles.
///
/// Reads the `particle_count`, `world_dimensions`, and `max_velocity` from the
/// `SimulationConfigResource` and uses `Commands` to spawn entities with
/// `ParticleId`, `Position`, and `Velocity` components. Positions and velocities
/// are initialized randomly within the world bounds and up to the maximum velocity.
/// This system is intended to run once when the `SimulationApp` starts.
///
/// # Arguments
///
/// * `commands` - Bevy `Commands` queue for spawning entities.
/// * `simulation_config` - The resource containing simulation configuration.
pub fn spawn_particles(
    mut commands: Commands,
    simulation_config: Res<SimulationConfigResource>,
) {
    let config = &simulation_config.0; // Get a reference to the inner config
    let (width, height) = config.world_dimensions;
    let max_vel = config.max_velocity; // Use max_velocity from config

    info!("Spawning {} particles...", config.particle_count);

    let mut rng = rand::thread_rng(); // Create RNG once

    for i in 0..config.particle_count {
        commands.spawn((
            ParticleId(i),
            Position {
                x: rng.gen::<f32>() * width, // Use rng
                y: rng.gen::<f32>() * height, // Use rng
            },
            Velocity {
                // Initialize with random velocity based on max_velocity
                dx: (rng.gen::<f32>() - 0.5) * max_vel * 2.0,
                dy: (rng.gen::<f32>() - 0.5) * max_vel * 2.0,
            },
        ));
    }

    info!("Particles spawned successfully");
}
