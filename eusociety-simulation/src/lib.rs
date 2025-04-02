use eusociety_core::World; // Position is used via world.storage.positions
use rand::Rng;

/// A simple system that iterates through all entities with a Position component
/// and applies a small random offset to their x and y coordinates.
pub fn random_movement_system(world: &mut World) {
    let mut rng = rand::thread_rng();
    for position in world.storage.positions.values_mut() {
        // Apply a small random change between -0.5 and 0.5
        position.x += rng.gen_range(-0.5..0.5);
        position.y += rng.gen_range(-0.5..0.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eusociety_core::World;

    #[test]
    fn test_random_movement() {
        let mut world = World::new();
        let e1 = world.create_entity(Position { x: 0.0, y: 0.0 });
        let initial_pos = *world.get_position(e1).unwrap();

        random_movement_system(&mut world);

        let final_pos = *world.get_position(e1).unwrap();

        // Check that the position has changed (highly likely with random movement)
        assert_ne!(initial_pos.x, final_pos.x);
        assert_ne!(initial_pos.y, final_pos.y);

        // Check that the change is within the expected bounds (e.g., not excessively large)
        assert!((final_pos.x - initial_pos.x).abs() <= 0.5);
        assert!((final_pos.y - initial_pos.y).abs() <= 0.5);
    }
}
