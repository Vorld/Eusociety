use crate::simulation::transport::{Transport, Serializer};
use crate::simulation::entity::Entity;
use crate::simulation::field::Field;
use tokio::sync::broadcast;
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug)]
pub struct WebSocketTransport {
    tx: broadcast::Sender<Vec<u8>>,
    serializer: Box<dyn Serializer>,
    max_chunk_size: usize,
}

impl WebSocketTransport {
    pub fn new(tx: broadcast::Sender<Vec<u8>>, serializer: Box<dyn Serializer>, max_chunk_size: usize) -> Self {
        Self {
            tx,
            serializer,
            max_chunk_size,
        }
    }
}

impl Transport for WebSocketTransport {
    fn init(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    
    fn send_state(&self, state: &[u8]) -> Result<(), Box<dyn Error>> {
        // Send in chunks if necessary
        for chunk in state.chunks(self.max_chunk_size) {
            if self.tx.receiver_count() > 0 {
                self.tx.send(chunk.to_vec())?;
            }
        }
        Ok(())
    }
    
    fn close(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    fn serialize_entities(&self, entities: &[Box<dyn Entity>]) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(entities.len() * 12);
        
        for entity in entities {
            buffer.extend_from_slice(&entity.serialize());
        }
        
        buffer
    }
    
    fn serialize_fields(&self, fields: &[Arc<dyn Field>]) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        // Add field type identifier and data for each field
        for field in fields {
            let field_type_id = match field.field_type() {
                "scalar" => 1u8,
                "vector" => 2u8,
                _ => 0u8,
            };
            
            buffer.push(field_type_id);
            buffer.extend_from_slice(&field.serialize());
        }
        
        buffer
    }
}