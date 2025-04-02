use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Basic Entity ID
pub type Entity = u32;

// Position Component
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

// Simplified Component Storage for Milestone 1
#[derive(Debug, Default, Serialize, Deserialize)] // Added Serialize/Deserialize for potential world state saving later
pub struct ComponentStorage {
    pub positions: HashMap<Entity, Position>,
}

// World containing the component storage
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct World {
    pub storage: ComponentStorage,
    // We can add global resources here later (e.g., delta_time, gravity)
    next_entity_id: Entity, // Simple way to generate new IDs if needed
}

impl World {
    pub fn new() -> Self {
        World::default()
    }

    // Helper to add an entity with a specific ID and position
    pub fn add_entity_with_position(&mut self, entity: Entity, position: Position) {
        self.storage.positions.insert(entity, position);
        // Ensure next_entity_id is always higher than the max used ID
        self.next_entity_id = self.next_entity_id.max(entity + 1);
    }

    // Helper to create a new entity with a position
    pub fn create_entity(&mut self, position: Position) -> Entity {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        self.storage.positions.insert(entity_id, position);
        entity_id
    }

    // Get a mutable reference to a position component
    pub fn get_position_mut(&mut self, entity: Entity) -> Option<&mut Position> {
        self.storage.positions.get_mut(&entity)
    }

     // Get an immutable reference to a position component
     pub fn get_position(&self, entity: Entity) -> Option<&Position> {
        self.storage.positions.get(&entity)
    }
}

// System function signature for Milestone 1
pub type System = fn(&mut World);

// Basic Scheduler for Milestone 1
#[derive(Default)]
pub struct Scheduler {
    systems: Vec<System>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler::default()
    }

    pub fn add_system(&mut self, system: System) {
        self.systems.push(system);
    }

    // Runs all systems sequentially
    pub fn run(&mut self, world: &mut World) {
        for system in &self.systems {
            system(world);
        }
    }
}

// Add basic tests
#[cfg(test)]
mod tests {
    use super::*;

    fn test_system(world: &mut World) {
        for pos in world.storage.positions.values_mut() {
            pos.x += 1.0;
        }
    }

    #[test]
    fn test_world_and_scheduler() {
        let mut world = World::new();
        let e1 = world.create_entity(Position { x: 0.0, y: 0.0 });
        let e2 = world.create_entity(Position { x: 10.0, y: 5.0 });

        assert_eq!(world.get_position(e1).unwrap().x, 0.0);
        assert_eq!(world.get_position(e2).unwrap().x, 10.0);

        let mut scheduler = Scheduler::new();
        scheduler.add_system(test_system);
        scheduler.run(&mut world);

        assert_eq!(world.get_position(e1).unwrap().x, 1.0);
        assert_eq!(world.get_position(e2).unwrap().x, 11.0);
    }
}
