use eusociety_core::{World, Position, DeltaTime, System, SystemAccess, AccessType}; // Removed unused DataAccess
// Removed unused Res import
use std::any::TypeId;
use rand::Rng;
use log::debug;

// Keep eusociety_macros::system import available for potential future use on simpler systems
#[allow(unused_imports)]
use eusociety_macros::system;

/// A system implementation that provides random movement to entities with a Position component
/// using the System trait directly
pub struct RandomMovementSystem;

impl System for RandomMovementSystem {
    fn access(&self) -> SystemAccess {
        SystemAccess::new()
            .with_component(TypeId::of::<Position>(), AccessType::Write)
            .with_resource(TypeId::of::<DeltaTime>(), AccessType::Read)
    }

    fn run(&mut self, world: &mut World) {
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
}

// Creating a simple manual system that uses Res<DeltaTime>
pub struct ResourceUsingSystem;

impl System for ResourceUsingSystem {
    fn access(&self) -> SystemAccess {
        SystemAccess::new()
            .with_component(TypeId::of::<Position>(), AccessType::Write) // Keep write access for now
            .with_resource(TypeId::of::<DeltaTime>(), AccessType::Read)
    }

    fn run(&mut self, world: &mut World) {
        let mut rng = rand::thread_rng();

        // Get the delta time for logging using the World's get_resource method
        if let Some(dt) = world.get_resource::<DeltaTime>() {
            debug!("ResourceUsingSystem Frame delta time: {:.6}s", dt.delta_seconds);

            // Use the original fixed random movement
            for (_, position) in world.components.query_mut::<Position>() {
                position.x += rng.gen_range(-0.1..0.1); // Different change
                position.y += rng.gen_range(-0.1..0.1);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use eusociety_core::{World, Position, System}; // Keep System for test setup
    use std::time::Duration;

    #[test]
    fn test_random_movement_system_manual_impl() {
        let mut world = World::new();

        let e1 = world.create_entity();
        world.add_component(e1, Position { x: 0.0, y: 0.0 });

        // Add DeltaTime resource
        world.insert_resource(DeltaTime::new(Duration::from_secs_f32(1.0 / 60.0)));

        let initial_pos = *world.get_component::<Position>(e1).unwrap();

        // Instantiate the manual system struct and run it
        let mut system_instance = RandomMovementSystem;
        system_instance.run(&mut world);

        let final_pos = *world.get_component::<Position>(e1).unwrap();

        // Basic check: position should change
        assert_ne!(initial_pos.x, final_pos.x);
        assert_ne!(initial_pos.y, final_pos.y);
    }
}
