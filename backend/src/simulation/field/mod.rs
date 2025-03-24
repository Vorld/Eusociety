use std::any::Any;
use serde::{Serialize, Deserialize};

pub mod scalar_field;

// Field trait for environmental influences
pub trait Field: Send + Sync {
    // Get field value at a position
    fn get_value(&self, x: f32, y: f32) -> FieldValue;
    
    // Modify field at a position
    fn add_value(&mut self, x: f32, y: f32, value: FieldValue);
    
    // Update field (e.g., diffusion, decay)
    fn update(&mut self, dt: f32);
    
    // Get field data for serialization
    fn serialize(&self) -> Vec<u8>;
    
    // Type information
    fn field_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Field value types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FieldValue {
    Scalar(f32),
    Vector(f32, f32),
}

// Field factory trait
pub trait FieldFactory: Send + Sync {
    fn create_field(&self, width: f32, height: f32, resolution: usize, 
                    properties: &serde_json::Value) -> Box<dyn Field>;
    fn field_type(&self) -> &'static str;
    fn clone_factory(&self) -> Box<dyn FieldFactory>;
}