use std::any::TypeId;
use std::collections::HashSet;
use crate::World;

use crate::ecs::system_param::SystemParam;
use std::marker::PhantomData;

/// Represents how a system accesses a component or resource
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    /// Read-only access
    Read,
    /// Read-write access
    Write,
}

/// Represents a single data dependency for a system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataAccess {
    /// The TypeId of the component or resource
    pub type_id: TypeId,
    /// How the system accesses this data
    pub access_type: AccessType,
}

impl DataAccess {
    /// Create a new DataAccess
    pub fn new(type_id: TypeId, access_type: AccessType) -> Self {
        Self { type_id, access_type }
    }
    
    /// Create a new read-only DataAccess
    pub fn read(type_id: TypeId) -> Self {
        Self { type_id, access_type: AccessType::Read }
    }
    
    /// Create a new read-write DataAccess
    pub fn write(type_id: TypeId) -> Self {
        Self { type_id, access_type: AccessType::Write }
    }
    
    /// Check if this access conflicts with another access
    pub fn conflicts_with(&self, other: &DataAccess) -> bool {
        self.type_id == other.type_id && 
        (self.access_type == AccessType::Write || other.access_type == AccessType::Write)
    }
}


// --- System Function Wrapper ---
// Use the existing PhantomData import, don't re-import it
//use std::marker::PhantomData;

/// A marker trait for types that can be used as system functions.
/// The `Marker` type parameter is used to distinguish implementations
/// for functions with different numbers of parameters.
pub trait IntoSystem<Params, Marker>: Send + Sync + 'static {
    /// The concrete system type created from this function.
    type System: System;
    /// Converts the function into a system.
    fn into_system(self) -> Self::System;
}

// --- One Parameter System ---
pub struct SystemFunction<F, P1>
where
    F: FnMut(P1::Item<'_, '_>) + Send + Sync + 'static, // Function takes the fetched param item
    P1: SystemParam,
{
    func: F,
    _marker: PhantomData<P1>,
}

impl<F, P1> System for SystemFunction<F, P1>
where
    F: FnMut(P1::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
{
    // The system's state is the state required by its parameter(s).
    type SystemState = P1::State;

    fn init_state(world: &mut World) -> Self::SystemState {
        P1::init_state(world)
    }

    fn access() -> SystemAccess {
        // Access is determined by the parameter(s).
        P1::access()
    }

    fn run(&mut self, world: &World, state: &mut Self::SystemState) {
        // 1. Fetch the parameter data using SystemParam::fetch
        let param = P1::fetch(world, state);

        // 2. Call the wrapped function with the fetched data.
        (self.func)(param);
    }

    // Inherit the name from the function type if possible, or use a default.
    fn name(&self) -> &str {
        std::any::type_name::<F>()
    }
}

// --- Two Parameters System ---
pub struct SystemFunction2<F, P1, P2>
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam,
    P2: SystemParam,
{
    func: F,
    _marker: PhantomData<(P1, P2)>,
}

impl<F, P1, P2> System for SystemFunction2<F, P1, P2>
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
    P2: SystemParam + Send + Sync,
{
    // The system's state is a tuple of the states required by its parameters.
    type SystemState = (P1::State, P2::State);

    fn init_state(world: &mut World) -> Self::SystemState {
        (P1::init_state(world), P2::init_state(world))
    }

    fn access() -> SystemAccess {
        // Combine access patterns from all parameters
        let mut access = P1::access();
        let access2 = P2::access();
        access.component_access.extend(access2.component_access);
        access.resource_access.extend(access2.resource_access);
        access
    }

    fn run(&mut self, world: &World, state: &mut Self::SystemState) {
        // Fetch parameters and call the function
        let param1 = P1::fetch(world, &mut state.0);
        let param2 = P2::fetch(world, &mut state.1);
        (self.func)(param1, param2);
    }

    fn name(&self) -> &str {
        std::any::type_name::<F>()
    }
}

// --- Three Parameters System ---
pub struct SystemFunction3<F, P1, P2, P3>
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>, P3::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam,
    P2: SystemParam,
    P3: SystemParam,
{
    func: F,
    _marker: PhantomData<(P1, P2, P3)>,
}

impl<F, P1, P2, P3> System for SystemFunction3<F, P1, P2, P3>
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>, P3::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
    P2: SystemParam + Send + Sync,
    P3: SystemParam + Send + Sync,
{
    // The system's state is a tuple of the states required by its parameters.
    type SystemState = (P1::State, P2::State, P3::State);

    fn init_state(world: &mut World) -> Self::SystemState {
        (P1::init_state(world), P2::init_state(world), P3::init_state(world))
    }

    fn access() -> SystemAccess {
        // Combine access patterns from all parameters
        let mut access = P1::access();
        let access2 = P2::access();
        let access3 = P3::access();
        access.component_access.extend(access2.component_access);
        access.component_access.extend(access3.component_access);
        access.resource_access.extend(access2.resource_access);
        access.resource_access.extend(access3.resource_access);
        access
    }

    fn run(&mut self, world: &World, state: &mut Self::SystemState) {
        // Fetch parameters and call the function
        let param1 = P1::fetch(world, &mut state.0);
        let param2 = P2::fetch(world, &mut state.1);
        let param3 = P3::fetch(world, &mut state.2);
        (self.func)(param1, param2, param3);
    }

    fn name(&self) -> &str {
        std::any::type_name::<F>()
    }
}

// --- Four Parameters System ---
pub struct SystemFunction4<F, P1, P2, P3, P4>
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>, P3::Item<'_, '_>, P4::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam,
    P2: SystemParam,
    P3: SystemParam,
    P4: SystemParam,
{
    func: F,
    _marker: PhantomData<(P1, P2, P3, P4)>,
}

impl<F, P1, P2, P3, P4> System for SystemFunction4<F, P1, P2, P3, P4>
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>, P3::Item<'_, '_>, P4::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
    P2: SystemParam + Send + Sync,
    P3: SystemParam + Send + Sync,
    P4: SystemParam + Send + Sync,
{
    // The system's state is a tuple of the states required by its parameters.
    type SystemState = (P1::State, P2::State, P3::State, P4::State);

    fn init_state(world: &mut World) -> Self::SystemState {
        (P1::init_state(world), P2::init_state(world), P3::init_state(world), P4::init_state(world))
    }

    fn access() -> SystemAccess {
        // Combine access patterns from all parameters
        let mut access = P1::access();
        let access2 = P2::access();
        let access3 = P3::access();
        let access4 = P4::access();
        access.component_access.extend(access2.component_access);
        access.component_access.extend(access3.component_access);
        access.component_access.extend(access4.component_access);
        access.resource_access.extend(access2.resource_access);
        access.resource_access.extend(access3.resource_access);
        access.resource_access.extend(access4.resource_access);
        access
    }

    fn run(&mut self, world: &World, state: &mut Self::SystemState) {
        // Fetch parameters and call the function
        let param1 = P1::fetch(world, &mut state.0);
        let param2 = P2::fetch(world, &mut state.1);
        let param3 = P3::fetch(world, &mut state.2);
        let param4 = P4::fetch(world, &mut state.3);
        (self.func)(param1, param2, param3, param4);
    }

    fn name(&self) -> &str {
        std::any::type_name::<F>()
    }
}

// --- IntoSystem implementations ---

// Implement IntoSystem for functions with one parameter
pub struct SystemParamFunction<F, P1>(PhantomData<(F, P1)>);

impl<F, P1> IntoSystem<P1, SystemParamFunction<F, P1>> for F
where
    F: FnMut(P1::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
{
    type System = SystemFunction<F, P1>;
    fn into_system(self) -> Self::System {
        SystemFunction {
            func: self,
            _marker: PhantomData,
        }
    }
}

// Implement IntoSystem for functions with two parameters
pub struct SystemParamFunction2<F, P1, P2>(PhantomData<(F, P1, P2)>);

impl<F, P1, P2> IntoSystem<(P1, P2), SystemParamFunction2<F, P1, P2>> for F
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
    P2: SystemParam + Send + Sync,
{
    type System = SystemFunction2<F, P1, P2>;
    fn into_system(self) -> Self::System {
        SystemFunction2 {
            func: self,
            _marker: PhantomData,
        }
    }
}

// Implement IntoSystem for functions with three parameters
pub struct SystemParamFunction3<F, P1, P2, P3>(PhantomData<(F, P1, P2, P3)>);

impl<F, P1, P2, P3> IntoSystem<(P1, P2, P3), SystemParamFunction3<F, P1, P2, P3>> for F
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>, P3::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
    P2: SystemParam + Send + Sync,
    P3: SystemParam + Send + Sync,
{
    type System = SystemFunction3<F, P1, P2, P3>;
    fn into_system(self) -> Self::System {
        SystemFunction3 {
            func: self,
            _marker: PhantomData,
        }
    }
}

// Implement IntoSystem for functions with four parameters
pub struct SystemParamFunction4<F, P1, P2, P3, P4>(PhantomData<(F, P1, P2, P3, P4)>);

impl<F, P1, P2, P3, P4> IntoSystem<(P1, P2, P3, P4), SystemParamFunction4<F, P1, P2, P3, P4>> for F
where
    F: FnMut(P1::Item<'_, '_>, P2::Item<'_, '_>, P3::Item<'_, '_>, P4::Item<'_, '_>) + Send + Sync + 'static,
    P1: SystemParam + Send + Sync,
    P2: SystemParam + Send + Sync,
    P3: SystemParam + Send + Sync,
    P4: SystemParam + Send + Sync,
{
    type System = SystemFunction4<F, P1, P2, P3, P4>;
    fn into_system(self) -> Self::System {
        SystemFunction4 {
            func: self,
            _marker: PhantomData,
        }
    }
}

/// Represents all data dependencies for a system
#[derive(Debug, Clone, Default)]
pub struct SystemAccess {
    /// Component dependencies
    pub component_access: Vec<DataAccess>,
    /// Resource dependencies
    pub resource_access: Vec<DataAccess>,
}

impl SystemAccess {
    /// Create a new empty SystemAccess
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a component access
    pub fn with_component(mut self, type_id: TypeId, access_type: AccessType) -> Self {
        self.component_access.push(DataAccess::new(type_id, access_type));
        self
    }
    
    /// Add a resource access
    pub fn with_resource(mut self, type_id: TypeId, access_type: AccessType) -> Self {
        self.resource_access.push(DataAccess::new(type_id, access_type));
        self
    }
    
    /// Check if this system's access conflicts with another system's access
    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        // Check for component access conflicts
        for my_access in &self.component_access {
            for other_access in &other.component_access {
                if my_access.conflicts_with(other_access) {
                    return true;
                }
            }
        }
        
        // Check for resource access conflicts
        for my_access in &self.resource_access {
            for other_access in &other.resource_access {
                if my_access.conflicts_with(other_access) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Get a set of TypeIds for all components that this system writes to
    pub fn component_writes(&self) -> HashSet<TypeId> {
        self.component_access
            .iter()
            .filter(|access| access.access_type == AccessType::Write)
            .map(|access| access.type_id)
            .collect()
    }
    
    /// Get a set of TypeIds for all resources that this system writes to
    pub fn resource_writes(&self) -> HashSet<TypeId> {
        self.resource_access
            .iter()
            .filter(|access| access.access_type == AccessType::Write)
            .map(|access| access.type_id)
            .collect()
    }
}

/// Core trait that all ECS systems must implement.
/// This version is designed to work with SystemParams and parallel execution.
pub trait System: Send + Sync + 'static {
    /// System-local state. Can be () if no state is needed.
    type SystemState: Send + Sync + 'static;

    /// Initializes the system's local state. Called once before the system runs for the first time.
    fn init_state(world: &mut World) -> Self::SystemState;

    /// Returns the system's component and resource dependencies.
    /// This is now an associated function, determined by the system type.
    fn access() -> SystemAccess;

    /// Executes the system logic.
    /// Takes an immutable reference to the world (due to interior mutability for params)
    /// and mutable references to self (for internal system state) and the system's local state.
    fn run(&mut self, world: &World, state: &mut Self::SystemState);

    /// Optional name for debugging and profiling.
    /// Remains an instance method if the name depends on instance data,
    /// or could become an associated function if static. Let's keep it as is for now.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}
