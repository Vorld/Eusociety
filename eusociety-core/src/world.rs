use std::collections::HashMap;
use crate::{Entity, component::Component};
use std::any::{Any, TypeId};
use std::cell::{RefCell, Ref, RefMut};

/// World stores entities and their components, and manages the execution of systems.
/// This implementation uses a component-major storage approach for better cache locality.
pub struct World {
    entities: HashMap<Entity, HashMap<TypeId, Box<dyn Any>>>,
    systems: Vec<Box<dyn crate::System>>,
    // Component storage: TypeId -> Vec<(Entity, Component)>
    // This gives us better cache locality when iterating components of the same type
    archetype_storage: HashMap<TypeId, Box<dyn Any>>,
    entity_manager: RefCell<crate::EntityManager>,
    resources: HashMap<TypeId, Box<dyn Any>>, // Global resources
}

impl World {
    pub fn new() -> Self {
        World {
            entities: HashMap::new(),
            systems: Vec::new(),
            archetype_storage: HashMap::new(),
            entity_manager: RefCell::new(crate::EntityManager::new()),
            resources: HashMap::new(),
        }
    }
    
    /// Create a new entity and return its identifier
    pub fn create_entity(&self) -> Entity {
        self.entity_manager.borrow_mut().create()
    }

    /// Spawn an entity with the provided entity ID
    pub fn spawn(&mut self, entity: Entity) {
        self.entities.insert(entity, HashMap::new());
    }

    /// Spawn a new entity and return its identifier
    pub fn spawn_entity(&mut self) -> Entity {
        let entity = self.create_entity();
        self.spawn(entity);
        entity
    }

    /// Add a component to an entity
    pub fn add_component<T: Component + 'static + Clone>(&mut self, entity: Entity, component: T) {
        // Store in entity-major storage
        if let Some(components) = self.entities.get_mut(&entity) {
            components.insert(
                TypeId::of::<T>(),
                Box::new(component.clone()),
            );
        }
        
        // Also store in component-major storage for more efficient queries
        let type_id = TypeId::of::<T>();
        let storage = self.archetype_storage
            .entry(type_id)
            .or_insert_with(|| Box::new(Vec::<(Entity, T)>::new()));
        
        if let Some(components) = storage.downcast_mut::<Vec<(Entity, T)>>() {
            // Check if entity already has this component type
            if let Some(idx) = components.iter().position(|(e, _)| *e == entity) {
                // Replace existing component
                components[idx].1 = component;
            } else {
                // Add new component
                components.push((entity, component));
            }
        }
    }

    /// Remove a component from an entity
    pub fn remove_component<T: Component + 'static>(&mut self, entity: Entity) {
        // Remove from entity-major storage
        if let Some(components) = self.entities.get_mut(&entity) {
            components.remove(&TypeId::of::<T>());
        }
        
        // Remove from component-major storage
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.archetype_storage.get_mut(&type_id) {
            if let Some(components) = storage.downcast_mut::<Vec<(Entity, T)>>() {
                if let Some(idx) = components.iter().position(|(e, _)| *e == entity) {
                    components.swap_remove(idx);
                }
            }
        }
    }

    /// Get a reference to a specific component for an entity
    pub fn get_component<T: Component + 'static>(&self, entity: Entity) -> Option<&T> {
        self.entities.get(&entity)
            .and_then(|components| components.get(&TypeId::of::<T>()))
            .and_then(|component| component.downcast_ref::<T>())
    }

    /// Get a mutable reference to a specific component for an entity
    pub fn get_component_mut<T: Component + 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.entities.get_mut(&entity)
            .and_then(|components| components.get_mut(&TypeId::of::<T>()))
            .and_then(|component| component.downcast_mut::<T>())
    }

    /// Query for all entities that have a specific component type
    pub fn query<T: Component + 'static>(&self) -> Vec<(&Entity, &T)> {
        let mut result = Vec::new();
        
        // Use component-major storage for more efficient iteration
        if let Some(storage) = self.archetype_storage.get(&TypeId::of::<T>()) {
            if let Some(components) = storage.downcast_ref::<Vec<(Entity, T)>>() {
                for (entity, component) in components {
                    result.push((entity, component));
                }
                return result;
            }
        }
        
        // Fallback to entity-major query
        for (entity, components) in &self.entities {
            if let Some(component) = components.get(&TypeId::of::<T>()) {
                if let Some(typed_component) = component.downcast_ref::<T>() {
                    result.push((entity, typed_component));
                }
            }
        }
        
        result
    }
    
    /// Query for all entities that have a specific component type and get mutable references
    pub fn query_mut<T: Component + 'static>(&mut self) -> Vec<(&Entity, &mut T)> {
        // This is more complex with archetype storage, so using entity-major approach for simplicity
        let mut result = Vec::new();
        
        for (entity, components) in &mut self.entities {
            if let Some(component) = components.get_mut(&TypeId::of::<T>()) {
                if let Some(typed_component) = component.downcast_mut::<T>() {
                    result.push((entity, typed_component));
                }
            }
        }
        
        result
    }

    /// Add a system to the world
    pub fn add_system<T: crate::System + 'static>(&mut self, system: T) {
        self.systems.push(Box::new(system));
    }

    /// Run all systems once
    pub fn run(&mut self) {
        // We need to avoid mutably borrowing self more than once
        // Clone the systems vec to avoid borrowing issues
        let mut systems = std::mem::take(&mut self.systems);
        
        for system in &mut systems {
            system.run(self);
        }
        
        // Put the systems back
        self.systems = systems;
    }

    /// Get the entity manager
    pub fn entity_manager(&self) -> Ref<crate::EntityManager> {
        self.entity_manager.borrow()
    }

    /// Get the entity manager mutably
    pub fn entity_manager_mut(&self) -> RefMut<crate::EntityManager> {
        self.entity_manager.borrow_mut()
    }
    
    /// Add a resource to the world
    pub fn add_resource<T: 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }
    
    /// Get a reference to a resource
    pub fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())
            .and_then(|res| res.downcast_ref::<T>())
    }
    
    /// Get a mutable reference to a resource
    pub fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources.get_mut(&TypeId::of::<T>())
            .and_then(|res| res.downcast_mut::<T>())
    }
    
    /// Remove a resource from the world
    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> {
        if let Some(resource) = self.resources.remove(&TypeId::of::<T>()) {
            if let Ok(typed_resource) = resource.downcast::<T>() {
                return Some(*typed_resource);
            }
        }
        None
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
