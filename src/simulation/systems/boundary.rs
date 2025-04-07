//! Contains the Bevy system for handling particle interactions with world boundaries.

use bevy_ecs::prelude::*;
use crate::config::BoundaryBehavior;
use crate::simulation::components::{Position, Velocity};
use crate::simulation::resources::SimulationConfigResource;

/// Bevy system that enforces world boundary behavior for particles.
///
/// Reads the `BoundaryBehavior` from the `SimulationConfigResource` and applies
/// either `Wrap` (teleporting to the opposite side) or `Bounce` (reversing velocity)
/// logic to particles that have moved outside the defined world dimensions.
/// Uses parallel iteration (`par_iter_mut`) for efficiency.
///
/// # Arguments
///
/// * `query` - A Bevy query to access mutable `Position` and `Velocity` components of particles.
/// * `simulation_config` - The resource containing simulation configuration, including world dimensions and boundary behavior.
pub fn handle_boundaries(
    mut query: Query<(&mut Position, &mut Velocity)>,
    simulation_config: Res<SimulationConfigResource>,
) {
    let (width, height) = simulation_config.0.world_dimensions;
    // Clone behavior outside the parallel iterator for thread safety
    let boundary_behavior = simulation_config.0.boundary_behavior.clone(); 

    // Use par_iter_mut for parallel processing
    query.par_iter_mut().for_each(|(mut pos, mut vel)| {
        match boundary_behavior { // Use the cloned value
            BoundaryBehavior::Wrap => {
                // Wrap around logic
                if pos.x < 0.0 { pos.x += width; }
                if pos.x >= width { pos.x -= width; }
                if pos.y < 0.0 { pos.y += height; }
                if pos.y >= height { pos.y -= height; }
            },
            BoundaryBehavior::Bounce => {
                // Bounce logic
                if pos.x < 0.0 || pos.x >= width {
                    vel.dx = -vel.dx;
                    pos.x = pos.x.clamp(0.0, width); // Clamp position after bounce
                }
                if pos.y < 0.0 || pos.y >= height {
                    vel.dy = -vel.dy;
                    pos.y = pos.y.clamp(0.0, height); // Clamp position after bounce
                }
            }
        }
    }); // Add semicolon after the for_each call
} // Function closing brace
