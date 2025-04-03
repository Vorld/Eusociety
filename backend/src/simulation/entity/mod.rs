pub mod particle; 

use serde::{Serialize, Deserialize};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;
use crate::simulation::field::Field;
use crate::simulation::config::WorldConfig;

// Backend - in entity/mod.rs or similar
/// Binary serialization format for entities:
/// - byte 0: Entity type (u8)
/// - bytes 1-4: Entity ID (u32, little-endian)
/// - bytes 5-8: X position (f32, little-endian)
/// - bytes 9-12: Y position (f32, little-endian)
/// Total size: 13 bytes per entity

// Entity trait defines the interface for all entities in the simulation
pub trait Entity: Send + Sync + Debug {
    // Core simulation methods
    fn update(&mut self, dt: f32, world: &WorldConfig, fields: &[Arc<dyn Field>]);
    fn interact_with(&mut self, other: &mut dyn Entity);
    
    // Spatial methods
    fn get_position(&self) -> (f32, f32);
    fn get_radius(&self) -> f32;
    
    // Type information
    fn entity_type(&self) -> EntityType;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    
    // Serialization
    fn serialize(&self) -> Vec<u8>;
}

// Base entity data shared by all entity types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub id: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub radius: f32,
}

// Entity types enum
// TODO: Consider using a more flexible system for entity types so I don't have to update this enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Particle = 0,
    Ant = 1,
    Food = 2,
    Nest = 3,
}

// Factory trait for creating entities
pub trait EntityFactory: Send + Sync {
    fn create_entity(&self, id: u32, x: f32, y: f32, properties: &serde_json::Value) 
        -> Box<dyn Entity>;
    fn entity_type(&self) -> EntityType;
    fn clone_factory(&self) -> Box<dyn EntityFactory>;
}