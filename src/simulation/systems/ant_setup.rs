//! System responsible for spawning the initial population of ants.

use bevy_ecs::prelude::*;
use crate::simulation::components::{Ant, Position, Velocity, ParticleId, AntState, PheromoneInfluence}; // Added PheromoneInfluence
use crate::simulation::resources::SimulationConfigResource;
use rand::{thread_rng, Rng};

const INITIAL_VELOCITY_MAGNITUDE: f32 = 0.0; // Initial speed of ants

/// System that runs once at startup to spawn the initial ants.
/// Ants are spawned randomly around the nest position (assumed to be 0,0 for now).
pub fn spawn_ants_system(
    mut commands: Commands,
    config: Res<SimulationConfigResource>, // Access simulation config
) {
    let mut rng = thread_rng();
    // Access the width using tuple index .0
    let world_width = config.0.world_dimensions.0;
    let spawn_radius = world_width / 4.0; // Spawn ants within a radius around the center

    for i in 0..config.0.particle_count {
        // Spawn ants near the center (nest)
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let radius = rng.gen_range(0.0..spawn_radius);
        let x = radius * angle.cos();
        let y = radius * angle.sin();

        // Assign random initial velocity
        let vel_angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let dx = INITIAL_VELOCITY_MAGNITUDE * vel_angle.cos();
        let dy = INITIAL_VELOCITY_MAGNITUDE * vel_angle.sin();

        commands.spawn((
            Ant { time_since_last_source: 0.0 }, // Initialize timer
            Position { x, y },
            Velocity { dx, dy },
            ParticleId(i), // Assign unique ID
            AntState::Foraging, // Start in Foraging state
            PheromoneInfluence::default(), // Initialize with zero influence
        ));
    }
}
