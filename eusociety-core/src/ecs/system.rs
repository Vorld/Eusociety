use std::any::TypeId;
use std::collections::HashSet;

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

/// Core trait that all ECS systems must implement
pub trait System: Send + Sync {
    /// Returns the system's component and resource dependencies
    fn access(&self) -> SystemAccess;
    
    /// Executes the system logic
    fn run(&mut self, world: &mut crate::World);
    
    /// Optional name for debugging and profiling
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}