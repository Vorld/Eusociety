use crate::ecs::system::{SystemAccess, DataAccess, AccessType};
use crate::ecs::world::World;
use crate::Component;
use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use crate::Entity;
use std::any::Any;
use std::collections::HashMap;
use crate::ComponentVec;
use crate::SystemParam;

// --- QueryFilter Trait ---
// Describes types that can be fetched by a Query.
pub unsafe trait QueryFilter: Send + Sync + 'static {
    /// The type of data fetched for a single entity (e.g., &'w Position, (&'w Pos, &'w mut Vel)).
    type Item<'w>;
    /// State needed by this filter (e.g., for change detection).
    type State: Send + Sync + 'static; 

    /// Initialize the state.
    fn init_state(world: &mut World) -> Self::State;

    /// Declare the component access required by this filter.
    fn access() -> SystemAccess;

    /// Fetch the data for a single entity from the locked component storages.
    unsafe fn fetch<'w>(
        guards: &FilteredComponentGuards<'w>,
        entity: Entity,
        state: &Self::State,
    ) -> Self::Item<'w>;
}

// --- FilteredComponentGuards (Helper) ---
// A structure to hold read and write guards for components
pub struct FilteredComponentGuards<'w> {
    // Maps component TypeId to either read or write guards
    read_guards: HashMap<TypeId, RwLockReadGuard<'w, Box<dyn Any + Send + Sync>>>,
    write_guards: HashMap<TypeId, RwLockWriteGuard<'w, Box<dyn Any + Send + Sync>>>,
    // Reference to world for accessing archetype entities
    world: &'w World,
}

impl<'w> FilteredComponentGuards<'w> {
    // Create a new set of guards based on the access pattern
    pub fn new(world: &'w World, access: &SystemAccess) -> Self {
        let mut read_guards = HashMap::new();
        let mut write_guards = HashMap::new();
        
        // Acquire all read guards first to avoid deadlocks
        for data_access in &access.component_access {
            if data_access.access_type == AccessType::Read {
                // This relies on the World having a method to get a component guard by TypeId
                if let Some(guard) = world.get_component_read_guard_by_id(data_access.type_id) {
                    read_guards.insert(data_access.type_id, guard);
                }
            }
        }
        
        // Then acquire all write guards
        for data_access in &access.component_access {
            if data_access.access_type == AccessType::Write {
                // This relies on the World having a method to get a component guard by TypeId
                if let Some(guard) = world.get_component_write_guard_by_id(data_access.type_id) {
                    write_guards.insert(data_access.type_id, guard);
                }
            }
        }
        
        Self {
            read_guards,
            write_guards,
            world,
        }
    }
    
    // Get the read guard for a component type
    pub fn get_read_guard<T: Component>(&self) -> Option<&RwLockReadGuard<'w, Box<dyn Any + Send + Sync>>> {
        let type_id = TypeId::of::<ComponentVec<T>>();
        self.read_guards.get(&type_id)
    }
    
    // Get the write guard for a component type
    pub fn get_write_guard<T: Component>(&self) -> Option<&RwLockWriteGuard<'w, Box<dyn Any + Send + Sync>>> {
        let type_id = TypeId::of::<ComponentVec<T>>();
        self.write_guards.get(&type_id)
    }
    
    // Get all entities that have all the required components
    pub fn matching_entities(&self) -> Vec<Entity> {
        // Collect the types we're interested in
        let mut type_ids: Vec<TypeId> = Vec::new();
        type_ids.extend(self.read_guards.keys().cloned());
        type_ids.extend(self.write_guards.keys().cloned());
        
        // Find entities that have all the required components
        if type_ids.is_empty() {
            return Vec::new();
        }
        
        self.world.find_entities_with_components(&type_ids)
    }
}


// --- QueryFilter Implementations ---

// Immutable fetch: &T with explicit 'static lifetime
unsafe impl<T: Component> QueryFilter for &'static T {
    type Item<'w> = &'w T;
    type State = ();

    fn init_state(_world: &mut World) -> Self::State { () }

    fn access() -> SystemAccess {
        SystemAccess::new()
            .with_component(TypeId::of::<T>(), AccessType::Read)
    }

    unsafe fn fetch<'w>(
        guards: &'w FilteredComponentGuards<'w>,
        entity: Entity,
        _state: &Self::State,
    ) -> Self::Item<'w> {
        // Get the component storage for this component type
        let type_id = TypeId::of::<ComponentVec<T>>();
        let guard = guards.read_guards.get(&type_id)
            .expect("Component read guard not found");
        
        // Downcast the guard to get the ComponentVec
        let storage = guard.downcast_ref::<ComponentVec<T>>()
            .expect("Component storage type mismatch");
        
        // Get the component reference for this entity
        let component_ref = storage.get(entity)
            .expect("Component not found for entity in &T query");
        
        // Return a reference with the correct lifetime
        component_ref
    }
}

// Mutable fetch: &mut T with explicit 'static lifetime
unsafe impl<T: Component> QueryFilter for &'static mut T {
    type Item<'w> = &'w mut T;
    type State = ();

    fn init_state(_world: &mut World) -> Self::State { () }

    fn access() -> SystemAccess {
        SystemAccess::new()
            .with_component(TypeId::of::<T>(), AccessType::Write)
    }

    unsafe fn fetch<'w>(
        guards: &'w FilteredComponentGuards<'w>,
        entity: Entity,
        _state: &Self::State,
    ) -> Self::Item<'w> {
        // Get the component storage for this component type
        let type_id = TypeId::of::<ComponentVec<T>>();
        let guard = guards.write_guards.get(&type_id)
            .expect("Component write guard not found");
        
        // Downcast the guard to get the ComponentVec by getting a mutable reference
        // to something that's immutable in a safe way
        let storage = {
            let ptr = &**guard as *const Box<dyn Any + Send + Sync>;
            let mut_ptr = ptr as *mut Box<dyn Any + Send + Sync>;
            &mut *(&mut *mut_ptr).downcast_mut::<ComponentVec<T>>().expect("Component storage type mismatch")
        };
        
        // Get the mutable component reference for this entity
        let component_ref = storage.get_mut(entity)
            .expect("Component not found for entity in &mut T query");
        
        // Return a mutable reference with the correct lifetime
        component_ref
    }
}

// --- QueryFilter Implementation for Tuples ---
// Using a macro would be better for more tuple sizes, but let's do (Q1, Q2) manually first.

unsafe impl<Q1, Q2> QueryFilter for (Q1, Q2)
where
    Q1: QueryFilter,
    Q2: QueryFilter,
{
    // The item is a tuple of the inner items
    type Item<'w> = (Q1::Item<'w>, Q2::Item<'w>);
    // State is a tuple of inner states
    type State = (Q1::State, Q2::State);

    fn init_state(world: &mut World) -> Self::State {
        (Q1::init_state(world), Q2::init_state(world))
    }

    fn access() -> SystemAccess {
        // Merge the access requirements.
        let mut access = Q1::access();
        let access2 = Q2::access();
        // TODO: This simple extend might add duplicates. Consider using a Set or merging logic.
        // Also, need proper conflict detection (e.g., &T and &mut T for same TypeId).
        // For now, rely on the scheduler's conflict detection based on the combined list.
        access.component_access.extend(access2.component_access);
        access.resource_access.extend(access2.resource_access);
        access
    }

    // Fetch data for both parts of the tuple
    // UNSAFE: Relies on the safety guarantees of the inner fetch implementations.
    unsafe fn fetch<'w>(
        guards: &FilteredComponentGuards<'w>,
        entity: Entity,
        state: &Self::State,
    ) -> Self::Item<'w> {
        (
            Q1::fetch(guards, entity, &state.0),
            Q2::fetch(guards, entity, &state.1),
        )
    }
}

// TODO: Implement QueryFilter for more tuple sizes (likely via macro)
// TODO: Implement QueryFilter for Option<Q>
// TODO: Implement QueryFilter for change detection wrappers (Added<T>, Changed<T>)


// --- Query SystemParam ---

/// System parameter to query entities with specific components.
/// F is the QueryFilter (e.g., &Position, (&Position, &mut Velocity)).
pub struct Query<'w, 's, F: QueryFilter> {
    // This struct needs to hold the state necessary to create the iterator.
    // It will likely hold references to the World's storages and the system's local state.
    world: &'w World,
    system_state: &'s F::State, // Use the state associated with the filter
    // PhantomData to tie lifetimes and the filter type
    _phantom: PhantomData<(&'w (), &'s (), F)>,
}

impl<'w, 's, F: QueryFilter> Query<'w, 's, F> {
    /// Creates a new Query instance. Called by SystemParam::fetch.
    pub(crate) fn new(world: &'w World, system_state: &'s F::State) -> Self {
        Self {
            world,
            system_state,
            _phantom: PhantomData,
        }
    }

    // Iterate over entities matching the query filter
    pub fn iter(&self) -> QueryIter<'w, 's, F> {
        let access = F::access();
        let guards = FilteredComponentGuards::new(self.world, &access);
        let entities = guards.matching_entities();
        
        QueryIter {
            guards,
            entities,
            current_index: 0,
            system_state: self.system_state,
            _phantom: PhantomData,
        }
    }
    
    // Get components for a specific entity
    pub fn get(&self, entity: Entity) -> Option<F::Item<'w>> {
        let access = F::access();
        let guards = FilteredComponentGuards::new(self.world, &access);
        
        // Check if entity has all required components
        if !self.world.has_all_components(entity, guards.read_guards.keys().chain(guards.write_guards.keys()).cloned().collect()) {
            return None;
        }
        
        // This is unsafe because we must ensure the entity has all components
        unsafe {
            Some(F::fetch(&guards, entity, self.system_state))
        }
    }
}

// Query iterator implementation
pub struct QueryIter<'w, 's, F: QueryFilter> {
    // Holds the component guards for the duration of iteration
    guards: FilteredComponentGuards<'w>,
    // List of entities that match the query
    entities: Vec<Entity>,
    // Current index in the entities list
    current_index: usize,
    // Reference to system state for the filter
    system_state: &'s F::State,
    // PhantomData to tie lifetimes and the filter type
    _phantom: PhantomData<F>,
}

impl<'w, 's, F: QueryFilter> Iterator for QueryIter<'w, 's, F> {
    type Item = F::Item<'w>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.entities.len() {
            return None;
        }
        
        let entity = self.entities[self.current_index];
        self.current_index += 1;
        
        // This is unsafe because we must ensure the entity has all components
        // We verified this when building the entities list
        unsafe {
            Some(F::fetch(&self.guards, entity, self.system_state))
        }
    }
}

// Import the SystemParam trait directly to fix the resolution error
use crate::SystemParam;

// Type-erased Query implementation that satisfies the 'static lifetime requirements
pub struct QuerySystemParam<F: QueryFilter>(pub PhantomData<F>);

// --- SystemParam Implementation for QuerySystemParam ---
impl<F: QueryFilter> SystemParam for QuerySystemParam<F> {
    type Item<'w, 's> = Query<'w, 's, F>;
    type State = F::State;

    fn init_state(world: &mut World) -> Self::State {
        F::init_state(world)
    }

    fn access() -> SystemAccess {
        F::access()
    }

    fn fetch<'w, 's>(world: &'w World, state: &'s mut Self::State) -> Self::Item<'w, 's> {
        Query::new(world, state)
    }
}
