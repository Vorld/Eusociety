use std::any::TypeId;
use crate::World; // Import World from crate root (lib.rs)
use crate::resources::{Res, ResMut, Resource}; // Use the existing Res/ResMut from resources.rs
use crate::ecs::system::{SystemAccess, AccessType};

// Create wrapper types that implement Send + Sync
pub struct ResState<T: Resource>(std::marker::PhantomData<T>);
pub struct ResMutState<T: Resource>(std::marker::PhantomData<T>);

/// A trait for types that can be fetched from the `World` as parameters for systems.
///
/// `'w` is the lifetime of the `World` borrow (or the lock guard).
/// `'s` is the lifetime of the `SystemState` borrow.
pub trait SystemParam: Sized + Send + Sync + 'static {
    /// The actual type fetched from the `World`.
    type Item<'w, 's>;

    /// Associated state required by this parameter.
    type State: Send + Sync + 'static;

    /// Initializes the state needed for this parameter.
    fn init_state(world: &mut World) -> Self::State;

    /// Declares the data access requirements of this parameter.
    fn access() -> SystemAccess;

    /// Fetches the data required by the parameter from the `World`.
    /// This requires careful handling of borrows and lifetimes.
    fn fetch<'w, 's>(world: &'w World, state: &'s mut Self::State) -> Self::Item<'w, 's>;
}

// --- Implementations for Resource Access ---

// We'll implement SystemParam for functions that take Res<T>
impl<T: Resource> SystemParam for fn(crate::resources::Res<'_, T>) {
    type Item<'w, 's> = crate::resources::Res<'w, T>;
    type State = ResState<T>;

    fn init_state(_world: &mut World) -> Self::State {
        ResState(std::marker::PhantomData)
    }

    fn access() -> SystemAccess {
        SystemAccess::new()
            .with_resource(TypeId::of::<T>(), AccessType::Read)
    }

    fn fetch<'w, 's>(world: &'w World, _state: &'s mut Self::State) -> Self::Item<'w, 's> {
        let guard = world.resources.get_read_guard::<T>()
            .expect("Resource not found");
        crate::resources::Res::new(guard)
    }
}

// We'll implement SystemParam for functions that take ResMut<T>
impl<T: Resource> SystemParam for fn(crate::resources::ResMut<'_, T>) {
    type Item<'w, 's> = crate::resources::ResMut<'w, T>;
    type State = ResMutState<T>;

    fn init_state(_world: &mut World) -> Self::State {
        ResMutState(std::marker::PhantomData)
    }

    fn access() -> SystemAccess {
        SystemAccess::new()
            .with_resource(TypeId::of::<T>(), AccessType::Write)
    }

    fn fetch<'w, 's>(world: &'w World, _state: &'s mut Self::State) -> Self::Item<'w, 's> {
        let guard = world.resources.get_write_guard::<T>()
            .expect("Resource not found");
        crate::resources::ResMut::new(guard)
    }
}

// TODO: Implement SystemParam for Query<'w, 's, F> (Task 3)
// TODO: Implement SystemParam for other useful types (e.g., Commands, Local<T>, EventReader/Writer)
