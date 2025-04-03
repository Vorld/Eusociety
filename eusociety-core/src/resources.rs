use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::marker::PhantomData; // Keep PhantomData if needed elsewhere, maybe not here anymore

/// Marker trait for types that can be stored as global resources in the World.
pub trait Resource: Any + Send + Sync {} // Resource implies Any + Send + Sync

// --- New Res implementation holding a Read Guard ---

/// A read-only handle to a resource, holding a lock guard.
///
/// This ensures the read lock is held while the `Res` exists.
pub struct Res<'a, T: Resource> {
    // Holds the guard, which in turn holds the lock and allows access to the Box<dyn Any>
    guard: RwLockReadGuard<'a, Box<dyn Any + Send + Sync>>,
    // PhantomData to link the lifetime 'a and type T
    _phantom: PhantomData<&'a T>,
}

impl<'a, T: Resource> Res<'a, T> {
    /// Creates a new `Res` from a read guard.
    /// This is typically called by `SystemParam::fetch`.
    pub(crate) fn new(guard: RwLockReadGuard<'a, Box<dyn Any + Send + Sync>>) -> Self {
        // Safety: The fetch implementation ensures the guard corresponds to type T.
        Self { guard, _phantom: PhantomData }
    }
}

impl<'a, T: Resource> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Downcast the Box<dyn Any> held by the guard to the concrete type T.
        // Panics if the downcast fails, which indicates a logic error in the fetch implementation.
        self.guard.downcast_ref::<T>().expect("Resource type mismatch in Res::deref. This is a bug in SystemParam fetch.")
    }
}

// --- New ResMut implementation holding a Write Guard ---

/// A read-write handle to a resource, holding a lock guard.
///
/// This ensures the write lock is held while the `ResMut` exists.
pub struct ResMut<'a, T: Resource> {
    // Holds the guard, which allows mutable access to the Box<dyn Any>
    guard: RwLockWriteGuard<'a, Box<dyn Any + Send + Sync>>,
    // PhantomData to link the lifetime 'a and type T
    _phantom: PhantomData<&'a mut T>,
}

impl<'a, T: Resource> ResMut<'a, T> {
    /// Creates a new `ResMut` from a write guard.
    /// This is typically called by `SystemParam::fetch`.
    pub(crate) fn new(guard: RwLockWriteGuard<'a, Box<dyn Any + Send + Sync>>) -> Self {
        // Safety: The fetch implementation ensures the guard corresponds to type T.
        Self { guard, _phantom: PhantomData }
    }
}

impl<'a, T: Resource> Deref for ResMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Downcast the Box<dyn Any> held by the guard.
        self.guard.downcast_ref::<T>().expect("Resource type mismatch in ResMut::deref. This is a bug in SystemParam fetch.")
    }
}

impl<'a, T: Resource> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Mutably downcast the Box<dyn Any> held by the guard.
        self.guard.downcast_mut::<T>().expect("Resource type mismatch in ResMut::deref_mut. This is a bug in SystemParam fetch.")
    }
}

// Removed obsolete ResourceParam and SystemParam trait definition from this file.
// The real SystemParam trait is in ecs/system_param.rs
