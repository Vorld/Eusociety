use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::any::{TypeId, Any};
pub use eusociety_macros::Component;

// Basic Entity ID
pub type Entity = u32;

// Component trait definition
pub trait Component: 'static + Send + Sync {
    fn type_id() -> TypeId where Self: Sized;
    fn type_name() -> &'static str where Self: Sized;
}

// Generic component storage using Vec<Option<T>>
#[derive(Debug)]
pub struct ComponentVec<T: Component> {
    data: Vec<Option<T>>,
}

impl<T: Component> Default for ComponentVec<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

impl<T: Component> ComponentVec<T> {
    pub fn insert(&mut self, entity: Entity, component: T) {
        let entity_idx = entity as usize;
        
        // Ensure the vector is large enough
        if entity_idx >= self.data.len() {
            self.data.resize_with(entity_idx + 1, || None);
        }
        
        self.data[entity_idx] = Some(component);
    }
    
    pub fn get(&self, entity: Entity) -> Option<&T> {
        let entity_idx = entity as usize;
        if entity_idx < self.data.len() {
            self.data[entity_idx].as_ref()
        } else {
            None
        }
    }
    
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let entity_idx = entity as usize;
        if entity_idx < self.data.len() {
            self.data[entity_idx].as_mut()
        } else {
            None
        }
    }
    
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let entity_idx = entity as usize;
        if entity_idx < self.data.len() {
            std::mem::take(&mut self.data[entity_idx])
        } else {
            None
        }
    }

    // Iterates over all entities that have this component
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(idx, opt)| opt.as_ref().map(|component| (idx as Entity, component)))
    }

    // Mutable iterator over all entities with this component
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.data
            .iter_mut()
            .enumerate()
            .filter_map(|(idx, opt)| opt.as_mut().map(|component| (idx as Entity, component)))
    }
}

// Position Component
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

// Type-erased component storage container
pub struct ComponentStorage {
    storages: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    entity_counter: Entity,
}

impl Default for ComponentStorage {
    fn default() -> Self {
        Self {
            storages: HashMap::new(),
            entity_counter: 0,
        }
    }
}

impl ComponentStorage {
    pub fn create_entity(&mut self) -> Entity {
        let entity = self.entity_counter;
        self.entity_counter += 1;
        entity
    }
    
    pub fn get_component_storage<T: Component>(&self) -> Option<&ComponentVec<T>> {
        self.storages.get(&TypeId::of::<ComponentVec<T>>())
            .and_then(|boxed| boxed.downcast_ref::<ComponentVec<T>>())
    }
    
    pub fn get_component_storage_mut<T: Component>(&mut self) -> &mut ComponentVec<T> {
        let type_id = TypeId::of::<ComponentVec<T>>();
        if !self.storages.contains_key(&type_id) {
            self.storages.insert(type_id, Box::new(ComponentVec::<T>::default()));
        }
        
        self.storages.get_mut(&type_id)
            .and_then(|boxed| boxed.downcast_mut::<ComponentVec<T>>())
            .unwrap()
    }
    
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let storage = self.get_component_storage_mut::<T>();
        storage.insert(entity, component);
    }
    
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get_component_storage::<T>()
            .and_then(|storage| storage.get(entity))
    }
    
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.get_component_storage_mut::<T>().get_mut(entity)
    }
    
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.get_component_storage_mut::<T>().remove(entity)
    }

    // Query functionality for finding entities with specific components
    pub fn query<T: Component>(&self) -> Box<dyn Iterator<Item = (Entity, &T)> + '_> {
        if let Some(storage) = self.get_component_storage::<T>() {
            Box::new(storage.iter())
        } else {
            Box::new(std::iter::empty())
        }
    }
    
    pub fn query_mut<T: Component>(&mut self) -> Box<dyn Iterator<Item = (Entity, &mut T)> + '_> {
        // Create the component storage if it doesn't exist yet
        let storage = self.get_component_storage_mut::<T>();
        Box::new(storage.iter_mut())
    }
    
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        self.get_component::<T>(entity).is_some()
    }
}

// Updated World with the new component storage
#[derive(Default)]
pub struct World {
    pub components: ComponentStorage,
    // We can add global resources here later (e.g., delta_time, gravity)
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn create_entity(&mut self) -> Entity {
        self.components.create_entity()
    }
    
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        self.components.add_component(entity, component);
    }
    
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.components.get_component(entity)
    }
    
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_component_mut(entity)
    }
    
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.components.remove_component(entity)
    }
}

// System function signature for Milestone 1 - keep for compatibility
pub type System = fn(&mut World);

// Basic Scheduler for Milestone 1 - keep for compatibility
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
        // Iterate over all positions and move them
        if let Some(storage) = world.components.get_component_storage_mut::<Position>() {
            for (_, pos) in storage.iter_mut() {
                pos.x += 1.0;
            }
        }
    }

    #[test]
    fn test_world_and_scheduler() {
        let mut world = World::new();
        let e1 = world.create_entity();
        world.add_component(e1, Position { x: 0.0, y: 0.0 });
        let e2 = world.create_entity();
        world.add_component(e2, Position { x: 10.0, y: 5.0 });

        assert_eq!(world.get_component::<Position>(e1).unwrap().x, 0.0);
        assert_eq!(world.get_component::<Position>(e2).unwrap().x, 10.0);

        let mut scheduler = Scheduler::new();
        scheduler.add_system(test_system);
        scheduler.run(&mut world);

        assert_eq!(world.get_component::<Position>(e1).unwrap().x, 1.0);
        assert_eq!(world.get_component::<Position>(e2).unwrap().x, 11.0);
    }

    #[test]
    fn test_generic_component() {
        // Define a new component type for testing
        #[derive(Debug, PartialEq, Component)]
        struct Health {
            value: i32,
        }

        let mut world = World::new();
        let entity = world.create_entity();
        
        world.add_component(entity, Health { value: 100 });
        world.add_component(entity, Position { x: 5.0, y: 10.0 });

        // Verify we can retrieve both component types
        assert_eq!(world.get_component::<Health>(entity).unwrap().value, 100);
        assert_eq!(world.get_component::<Position>(entity).unwrap().x, 5.0);

        // Test component mutation
        if let Some(health) = world.get_component_mut::<Health>(entity) {
            health.value -= 30;
        }
        
        assert_eq!(world.get_component::<Health>(entity).unwrap().value, 70);
    }
}
