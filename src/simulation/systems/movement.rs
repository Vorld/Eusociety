//! Contains the Bevy system responsible for updating particle positions based on velocity.

use bevy_ecs::prelude::*;
use crate::simulation::components::{Position, Velocity};
use crate::simulation::resources::Time;

/// Bevy system that updates the `Position` of each particle based on its `Velocity` and the frame's `delta_seconds`.
///
/// Uses parallel iteration (`par_iter_mut`) for efficiency.
///
/// # Arguments
///
/// * `time` - The `Time` resource providing delta time information.
/// * `query` - A Bevy query to access mutable `Position` and immutable `Velocity` components of particles.
pub fn move_particles(
    time: Res<Time>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    // Using par_for_each for parallel iteration over all particles
    query.par_iter_mut().for_each(|(mut position, velocity)| {
        position.x += velocity.dx * time.delta_seconds;
        position.y += velocity.dy * time.delta_seconds;
    });
}
