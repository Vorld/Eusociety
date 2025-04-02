use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::any::{TypeId, Any};
use std::time::Duration;
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

/// Marker trait for types that can be stored as global resources in the World.
/// 
/// Resources represent global shared state that can be accessed by systems,
/// as opposed to components which are associated with specific entities.
/// Only one instance of a resource type can exist in the World at any time.
/// 
/// # Examples of appropriate resources:
/// - Game time / delta time
/// - Physics constants
/// - Global game state
/// - Asset managers
/// 
/// Resources must be `Send + Sync` to ensure thread-safety for future
/// parallel system execution.
pub trait Resource: 'static + Send + Sync {}

/// A resource that tracks the time elapsed between frames
#[derive(Debug, Clone, Copy)]
pub struct DeltaTime {
    /// The duration of the last frame in seconds
    pub delta_seconds: f32,
    /// The raw duration object
    pub delta: Duration,
}

impl Default for DeltaTime {
    fn default() -> Self {
        Self {
            delta_seconds: 0.0,
            delta: Duration::from_secs(0),
        }
    }
}

impl DeltaTime {
    /// Create a new DeltaTime from a Duration
    pub fn new(duration: Duration) -> Self {
        Self {
            delta_seconds: duration.as_secs_f32(),
            delta: duration,
        }
    }
    
    /// Update the DeltaTime with a new Duration
    pub fn update(&mut self, duration: Duration) {
        self.delta = duration;
        self.delta_seconds = duration.as_secs_f32();
    }
}

impl Resource for DeltaTime {}

// Resource storage container
pub struct ResourceStorage {
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Default for ResourceStorage {
    fn default() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }
}

impl ResourceStorage {
    pub fn insert<T: Resource>(&mut self, resource: T) {
        let type_id = TypeId::of::<T>();
        self.resources.insert(type_id, Box::new(resource));
    }
    
    pub fn get<T: Resource>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.resources.get(&type_id)
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }
    
    pub fn get_mut<T: Resource>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.resources.get_mut(&type_id)
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }
    
    pub fn remove<T: Resource>(&mut self) -> Option<T> {
        let type_id = TypeId::of::<T>();
        if let Some(boxed) = self.resources.remove(&type_id) {
            boxed.downcast().ok().map(|boxed| *boxed)
        } else {
            None
        }
    }
    
    pub fn contains<T: Resource>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.resources.contains_key(&type_id)
    }
}

// Updated World with component and resource storage
#[derive(Default)]
pub struct World {
    pub components: ComponentStorage,
    pub resources: ResourceStorage,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Entity and component methods
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
    
    // Resource methods
    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources.insert(resource);
    }
    
    pub fn get_resource<T: Resource>(&self) -> Option<&T> {
        self.resources.get::<T>()
    }
    
    pub fn get_resource_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.resources.get_mut::<T>()
    }
    
    pub fn remove_resource<T: Resource>(&mut self) -> Option<T> {
        self.resources.remove::<T>()
    }
    
    pub fn has_resource<T: Resource>(&self) -> bool {
        self.resources.contains::<T>()
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
        for (_, pos) in world.components.query_mut::<Position>() {
            pos.x += 1.0;
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

    #[test]
    fn test_resource_management() {
        #[derive(Debug, Clone, PartialEq)]
        struct TestResource {
            value: i32,
        }
        
        impl Resource for TestResource {}
        
        let mut world = World::new();
        
        // Test inserting and retrieving a resource
        world.insert_resource(TestResource { value: 42 });
        assert_eq!(world.get_resource::<TestResource>().unwrap().value, 42);
        
        // Test modifying a resource
        if let Some(res) = world.get_resource_mut::<TestResource>() {
            res.value = 100;
        }
        assert_eq!(world.get_resource::<TestResource>().unwrap().value, 100);
        
        // Test has_resource
        assert!(world.has_resource::<TestResource>());
        assert!(!world.has_resource::<DeltaTime>());
        
        // Test removing a resource
        let removed = world.remove_resource::<TestResource>().unwrap();
        assert_eq!(removed.value, 100);
        assert!(!world.has_resource::<TestResource>());
        
        // Test inserting DeltaTime resource
        let dt = DeltaTime::new(Duration::from_millis(16));
        world.insert_resource(dt);
        assert!(world.has_resource::<DeltaTime>());
        assert_eq!(world.get_resource::<DeltaTime>().unwrap().delta_seconds, 0.016);
    }
}
