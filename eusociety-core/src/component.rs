use std::any::Any;

/// Component trait for all components in the ECS system
/// 
/// Components should be pure data with no behavior
pub trait Component: Send + Sync + 'static {
    /// Get the component as Any for dynamic downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Get the component as mutable Any for dynamic downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
    
    /// Clone the component
    fn clone_box(&self) -> Box<dyn Component>;
}

/// Macro to automatically implement Component for a struct
#[macro_export]
macro_rules! impl_component {
    ($component:ty) => {
        impl Component for $component {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            
            fn clone_box(&self) -> Box<dyn Component> {
                Box::new(self.clone())
            }
        }
    };
}
