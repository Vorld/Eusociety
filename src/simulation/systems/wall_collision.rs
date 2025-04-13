//! System responsible for handling collisions between ants and polygon walls.

use bevy_ecs::prelude::*;
use glam::Vec2; // Use glam for vector math
use crate::simulation::components::{Position, Velocity};
use crate::simulation::resources::{WallGeometry, Time}; // Import Time resource

const COLLISION_EPSILON: f32 = 1e-6; // Small value for float comparisons and pushback

/// Bevy system that detects and handles collisions between ants and defined polygon walls.
///
/// Iterates through each ant and checks its movement path against all wall segments.
/// If a collision is detected, it adjusts the ant's velocity (bouncing) and position
/// to prevent penetration.
pub fn handle_wall_collisions(
    mut query: Query<(Entity, &mut Position, &mut Velocity)>, // Added Entity for logging
    walls: Res<WallGeometry>,
    time: Res<Time>, // Access delta time
) {
    if walls.polygons.is_empty() {
        return; // No walls to collide with
    }

    let delta_time = time.delta_seconds;
    if delta_time <= 0.0 { return; } // Avoid division by zero or weirdness

    // Use .iter_mut() instead of .par_iter_mut() for now, as parallel collision
    // resolution can be tricky if multiple ants collide with the same wall segment
    // simultaneously. Can revisit parallelism if performance requires it.
    for (_entity, mut pos, mut vel) in query.iter_mut() {
        let current_pos = Vec2::new(pos.x, pos.y);
        let current_vel = Vec2::new(vel.dx, vel.dy);

        // Calculate potential next position based on current velocity
        // This defines the ant's movement segment for this frame
        let potential_next_pos = current_pos + current_vel * delta_time;
        let movement_segment = (current_pos, potential_next_pos);

        // Store the closest collision found so far for this ant
        let mut closest_collision: Option<(Vec2, Vec2)> = None; // (intersection_point, wall_normal)
        let mut min_dist_sq = f32::MAX;

        for polygon in &walls.polygons {
            let num_vertices = polygon.vertices.len();
            if num_vertices < 2 { continue; }

            for i in 0..num_vertices {
                let p1_config = &polygon.vertices[i];
                let p2_config = &polygon.vertices[(i + 1) % num_vertices]; // Wrap around

                let p1 = Vec2::new(p1_config.x, p1_config.y);
                let p2 = Vec2::new(p2_config.x, p2_config.y);
                let wall_segment = (p1, p2);

                // Check for intersection
                if let Some(intersection) = intersect_segment_segment(movement_segment, wall_segment) {
                    let dist_sq = current_pos.distance_squared(intersection);

                    // If this is the first collision or closer than the previous one
                    if dist_sq < min_dist_sq {
                        min_dist_sq = dist_sq;

                        // Calculate wall normal (pointing outwards from the segment)
                        let wall_vec = p2 - p1;
                        // Ensure normal points consistently (e.g., assuming clockwise polygon vertices)
                        // A simple perpendicular vector:
                        let normal = Vec2::new(wall_vec.y, -wall_vec.x).normalize_or_zero();

                        closest_collision = Some((intersection, normal));
                    }
                }
            }
        }

        // Handle the closest collision found for this ant
        if let Some((intersection_point, wall_normal)) = closest_collision {
            // Reflect velocity
            let reflected_vel = current_vel.reflect(wall_normal);

            // Apply damping (optional - using boundary.rs style for now)
            // TODO: Make damping configurable?
            let damping = 0.10; // Match boundary.rs bounce damping
            vel.dx = reflected_vel.x * damping;
            vel.dy = reflected_vel.y * damping;

            // Set position exactly to the collision point, slightly pushed back
            // along the normal to prevent sticking/re-collision immediately.
            let pushback_pos = intersection_point + wall_normal * COLLISION_EPSILON;
            pos.x = pushback_pos.x;
            pos.y = pushback_pos.y;

            // Optional: Log collision details
            // tracing::trace!(?entity, pos = ?pushback_pos, vel = ?(vel.dx, vel.dy), "Wall collision");
        }
        // If no collision, the position will be updated later by the move_particles system
        // based on the original velocity.
    }
}

/// Checks if two line segments intersect.
/// Segment 1: (a, b)
/// Segment 2: (c, d)
/// Returns the intersection point `Some(Vec2)` if they intersect, `None` otherwise.
fn intersect_segment_segment(seg1: (Vec2, Vec2), seg2: (Vec2, Vec2)) -> Option<Vec2> {
    let (a, b) = seg1;
    let (c, d) = seg2;

    let v1 = b - a; // Vector from a to b
    let v2 = d - c; // Vector from c to d
    let v3 = c - a; // Vector from a to c

    // Using perp_dot (2D cross product): a.perp_dot(b) = a.x * b.y - a.y * b.x
    let denominator = v1.perp_dot(v2);

    // Check if lines are parallel or collinear
    if denominator.abs() < COLLISION_EPSILON {
        // TODO: Handle collinear overlapping segments if needed (complex case)
        return None; // Treat parallel lines as non-intersecting for simplicity
    }

    // Calculate intersection parameters t and u
    // Intersection point P = a + t * v1 = c + u * v2
    let t = v3.perp_dot(v2) / denominator;
    let u = v3.perp_dot(v1) / denominator;

    // Check if the intersection point lies within both segments
    if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
        // Intersection point found
        let intersection = a + v1 * t;
        Some(intersection)
    } else {
        // Intersection point is outside one or both segments
        None
    }
}