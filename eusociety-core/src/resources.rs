use std::marker::PhantomData;

/// Marker trait for types that can be stored as global resources in the World.
/// Resources represent global shared state that can be accessed by systems.
pub trait Resource: 'static + Send + Sync {}

/// A read-only handle to a resource
pub struct Res<'a, T: Resource> {
    value: &'a T,
}

impl<'a, T: Resource> Res<'a, T> {
    /// Create a new Res handle
    pub fn new(value: &'a T) -> Self { // Changed to pub
        Self { value }
    }
}

impl<'a, T: Resource> std::ops::Deref for Res<'a, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

/// A read-write handle to a resource
pub struct ResMut<'a, T: Resource> {
    value: &'a mut T,
}

impl<'a, T: Resource> ResMut<'a, T> {
    /// Create a new ResMut handle
    pub fn new(value: &'a mut T) -> Self { // Changed to pub
        Self { value }
    }
}

impl<'a, T: Resource> std::ops::Deref for ResMut<'a, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T: Resource> std::ops::DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

/// A marker type that tracks a resource type for system parameters
pub struct ResourceParam<T: Resource, const MUTABLE: bool> {
    _phantom: PhantomData<T>,
}

/// Trait for types that can be used as system parameters
pub trait SystemParam {
    // Will be expanded in future milestones
}

// Implement SystemParam for ResourceParam
impl<T: Resource, const MUTABLE: bool> SystemParam for ResourceParam<T, MUTABLE> {}
