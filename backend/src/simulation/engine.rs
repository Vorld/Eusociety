use crate::simulation::entity::Entity;
use crate::simulation::field::Field;
use crate::simulation::config::{SimulationConfig, WorldConfig};
use crate::simulation::registry::REGISTRY;
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;

pub struct SimulationEngine {
    pub entities: Vec<Box<dyn Entity + Send + Sync>>,
    pub fields: Vec<Arc<dyn Field + Send + Sync>>,
    pub config: SimulationConfig,
    pub batch_size: usize,
}

impl SimulationEngine {
    pub fn new(batch_size: usize) -> Self {
        Self {
            entities: Vec::new(),
            fields: Vec::new(),
            config: SimulationConfig::default(),
            batch_size,
        }
    }
    
    pub fn with_config(mut self, config: SimulationConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn initialize(&mut self) {
        // Clear existing entities and fields
        self.entities.clear();
        self.fields.clear();
        
        // Create fields based on config
        for field_config in &self.config.field_configs {
            if let Some(factory) = REGISTRY.get_field_factory(&field_config.field_type) {
                let field = factory.create_field(
                    self.config.world.width,
                    self.config.world.height,
                    field_config.resolution,
                    &serde_json::to_value(&field_config.properties).unwrap_or_default(),
                );
                let field: Box<dyn Field + Send + Sync> = field;
                self.fields.push(Arc::from(field));
            }
        }
        
        // Create entities based on config
        let mut next_id = 0;
        for entity_config in &self.config.entity_configs {
            if let Some(factory) = REGISTRY.get_entity_factory(&entity_config.entity_type) {
                for _ in 0..entity_config.count {
                    // Generate position based on entity type and count
                    let x = rand::random::<f32>() * self.config.world.width;
                    let y = rand::random::<f32>() * self.config.world.height;
                    
                    let entity = factory.create_entity(
                        next_id,
                        x, y,
                        &serde_json::to_value(&entity_config.properties).unwrap_or_default(),
                    );
                    
                    self.entities.push(entity);
                    next_id += 1;
                }
            }
        }
    }
    
    pub fn update(&mut self, dt: f32) {
        // Update fields first
        for field in &mut self.fields {
            // Need to get mutable access to the Arc
            let field_ref = Arc::get_mut(field).expect("Failed to get mutable reference to field");
            field_ref.update(dt);
        }
        
        // Update entities in parallel
        let world_config = self.config.world.clone();
        let fields_ref = &self.fields;
        
        self.entities.par_chunks_mut(self.batch_size).for_each(|chunk| {
            for entity in chunk {
                entity.update(
                    dt,
                    &world_config,
                    unsafe {
                        std::mem::transmute::<&[Arc<dyn Field + Send + Sync>], &[Arc<dyn Field>]>(fields_ref.as_slice())
                    }
                );
            }
        });
        
        // Entity interactions - simplified approach
        // For more complex simulations, you'd want spatial partitioning
        // This is O(n²) and not efficient for large simulations
        if self.entities.len() < 1000 {  // Only do interactions for smaller simulations
            for i in 0..self.entities.len() {
                for j in (i+1)..self.entities.len() {
                    // Need safe way to get mutable refs to two elements
                    let (left, right) = self.entities.split_at_mut(j);
                    let entity1 = &mut left[i];
                    let entity2 = &mut right[0];
                    
                    entity1.interact_with(entity2.as_mut());
                    entity2.interact_with(entity1.as_mut());
                }
            }
        }
    }
    
    pub fn serialize_state(&self, buffer: &mut Vec<u8>) {
        // Serialize entities
        for entity in &self.entities {
            buffer.extend_from_slice(&entity.serialize());
        }
    }
}