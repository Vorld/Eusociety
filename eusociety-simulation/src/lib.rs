use eusociety_core::{World, Position, DeltaTime}; // Import Position and DeltaTime directly
use rand::Rng;
use log::debug;

/// A simple system that iterates through all entities with a Position component
/// and applies a small random offset to their x and y coordinates.
/// The system logs the delta time but does not use it for movement calculations.
pub fn random_movement_system(world: &mut World) {
    let mut rng = rand::thread_rng();
    
    // Get the delta time for logging purposes
    if let Some(dt) = world.get_resource::<DeltaTime>() {
        debug!("Frame delta time: {:.6}s", dt.delta_seconds);
    } else {
        debug!("DeltaTime resource not found");
    }
    
    // Use the original fixed random movement (not scaled by delta time)
    for (_, position) in world.components.query_mut::<Position>() {
        // Apply a small random change between -0.5 and 0.5
        position.x += rng.gen_range(-0.5..0.5);
        position.y += rng.gen_range(-0.5..0.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eusociety_core::{World, Position};
    use std::time::Duration;

    #[test]
    fn test_random_movement() {
        let mut world = World::new();

        let e1 = world.create_entity();
        world.add_component(e1, Position { x: 0.0, y: 0.0 });
        
        // Add DeltaTime resource with a fixed value for demonstration purposes
        world.insert_resource(DeltaTime::new(Duration::from_secs_f32(1.0 / 60.0)));
        
        let initial_pos = *world.get_component::<Position>(e1).unwrap();

        random_movement_system(&mut world);

        let final_pos = *world.get_component::<Position>(e1).unwrap();

        // Check that the position has changed (highly likely with random movement)
        assert_ne!(initial_pos.x, final_pos.x);
        assert_ne!(initial_pos.y, final_pos.y);

        // Check that the change is within the expected bounds
        assert!((final_pos.x - initial_pos.x).abs() <= 0.5);
        assert!((final_pos.y - initial_pos.y).abs() <= 0.5);
    }
}
