use tokio::sync::broadcast;
use std::error::Error;
use std::fmt::Debug;

pub mod websocket;

// Transport trait for sending simulation state to clients
pub trait Transport: Send + Sync + Debug {
    fn init(&mut self) -> Result<(), Box<dyn Error>>;
    fn send_state(&self, state: &[u8]) -> Result<(), Box<dyn Error>>;
    fn close(&mut self) -> Result<(), Box<dyn Error>>;
}

// Serializer trait for different output formats
pub trait Serializer: Send + Sync + Debug {
    fn serialize_entities(&self, entities: &[Box<dyn crate::simulation::entity::Entity>]) -> Vec<u8>;
    fn serialize_fields(&self, fields: &[std::sync::Arc<dyn crate::simulation::field::Field>]) -> Vec<u8>;
}