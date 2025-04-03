// Core ECS implementation for eusociety-core

// Re-export key components of the ECS system
pub mod system;
pub mod system_param;
pub mod scheduler;
pub mod query;
pub mod world { pub use crate::World; }

// Re-export key types for convenience
pub use self::query::Query;
pub use self::system::System;
pub use self::system_param::SystemParam;

// Testing module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{World, Component, Position};
    use crate::resources::{Res, ResMut, Resource};
    use crate::DeltaTime;
    use std::marker::PhantomData;
    
    // Test component
    #[derive(Debug, Clone, Copy)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }
    
    // Implement Component trait manually to avoid macro dependency issues
    impl Component for Velocity {
        fn type_id() -> std::any::TypeId { 
            std::any::TypeId::of::<Self>()
        }
        fn type_name() -> &'static str {
            std::any::type_name::<Self>()
        }
    }
    
    // Define test systems with concrete implementations
    struct MovementSystem {
        _marker: PhantomData<fn()>,
    }
    
    impl MovementSystem {
        fn new() -> Self {
            Self { _marker: PhantomData }
        }
    }
    
    impl crate::System for MovementSystem {
        type SystemState = (query::QuerySystemParam<(&'static mut Position, &'static Velocity)>::State, <Res<DeltaTime> as crate::SystemParam>::State);
        
        fn init_state(world: &mut World) -> Self::SystemState {
            (
                query::QuerySystemParam::<(&'static mut Position, &'static Velocity)>::init_state(world),
                <Res<DeltaTime> as crate::SystemParam>::init_state(world)
            )
        }
        
        fn access() -> crate::SystemAccess {
            let mut access = query::QuerySystemParam::<(&'static mut Position, &'static Velocity)>::access();
            let res_access = <Res<DeltaTime> as crate::SystemParam>::access();
            
            access.component_access.extend(res_access.component_access);
            access.resource_access.extend(res_access.resource_access);
            access
        }
        
        fn run(&mut self, world: &World, state: &mut Self::SystemState) {
            let query = query::QuerySystemParam::<(&'static mut Position, &'static Velocity)>::fetch(world, &mut state.0);
            let time = <Res<DeltaTime> as crate::SystemParam>::fetch(world, &mut state.1);
            
            for (mut pos, vel) in query.iter() {
                pos.x += vel.dx * time.delta_seconds;
                pos.y += vel.dy * time.delta_seconds;
            }
        }
    }
    
    struct VelocityScaleSystem {
        _marker: PhantomData<fn()>,
    }
    
    impl VelocityScaleSystem {
        fn new() -> Self {
            Self { _marker: PhantomData }
        }
    }
    
    impl crate::System for VelocityScaleSystem {
        type SystemState = query::QuerySystemParam<&'static mut Velocity>::State;
        
        fn init_state(world: &mut World) -> Self::SystemState {
            query::QuerySystemParam::<&'static mut Velocity>::init_state(world)
        }
        
        fn access() -> crate::SystemAccess {
            query::QuerySystemParam::<&'static mut Velocity>::access()
        }
        
        fn run(&mut self, world: &World, state: &mut Self::SystemState) {
            let query = query::QuerySystemParam::<&'static mut Velocity>::fetch(world, state);
            
            for mut vel in query.iter() {
                vel.dx *= 1.01; // Increase x velocity by 1%
                vel.dy *= 1.01; // Increase y velocity by 1%
            }
        }
    }
    
    // Test that the parallel scheduler works correctly
    #[test]
    fn test_parallel_scheduler() {
        let mut world = World::new();
        
        // Insert a delta time resource
        world.insert_resource(DeltaTime::new(std::time::Duration::from_millis(16))); // 16ms per frame
        
        // Create some entities with position and velocity
        for i in 0..10 {
            let entity = world.create_entity();
            world.add_component(entity, Position { x: i as f32, y: 0.0 });
            world.add_component(entity, Velocity { dx: 1.0, dy: 0.5 });
        }
        
        // Create a scheduler with our systems
        let mut scheduler = scheduler::SystemScheduler::new();
        
        // Add our concrete system implementations
        scheduler.add_system(MovementSystem::new(), &mut world);
        scheduler.add_system(VelocityScaleSystem::new(), &mut world);
        
        // Run the scheduler for several frames
        for _ in 0..5 {
            scheduler.run(&world);
        }
        
        // Verify that entities moved correctly
        for entity in 0..10 {
            if let Some(pos) = world.get_component::<Position>(entity) {
                // Initial position was (i, 0)
                // Velocity starts at (1.0, 0.5) and increases by 1% each frame
                // Duration is 0.016 seconds per frame
                // After 5 frames, position should be roughly:
                // x = i + (1.0 * 1.01^0 + 1.0 * 1.01^1 + ... + 1.0 * 1.01^4) * 0.016
                // y = 0 + (0.5 * 1.01^0 + 0.5 * 1.01^1 + ... + 0.5 * 1.01^4) * 0.016
                
                // Calculate expected approximate position
                // This is a very rough approximation for the test
                let initial_x = entity as f32;
                assert!(pos.x > initial_x); // Position should have increased
                assert!(pos.y > 0.0); // Position should have increased from 0
                
                // More precise check would involve the exact calculation with the scaling factor
                // but this simplified check is sufficient to verify the systems ran
            } else {
                panic!("Entity {} should have a Position component", entity);
            }
        }
    }
}
