use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::simulation::entity::{EntityFactory, EntityType};
use crate::simulation::field::FieldFactory;
use once_cell::sync::Lazy;

// Global registry for entity and field factories
pub static REGISTRY: Lazy<Registry> = Lazy::new(|| Registry::new());

pub struct Registry {
    entity_factories: RwLock<HashMap<String, Box<dyn EntityFactory>>>,
    field_factories: RwLock<HashMap<String, Box<dyn FieldFactory>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            entity_factories: RwLock::new(HashMap::new()),
            field_factories: RwLock::new(HashMap::new()),
        }
    }
    
    pub fn register_entity_factory(&self, name: &str, factory: Box<dyn EntityFactory>) {
        let mut factories = self.entity_factories.write().unwrap();
        factories.insert(name.to_string(), factory);
    }
    
    pub fn get_entity_factory(&self, name: &str) -> Option<Box<dyn EntityFactory>> {
        let factories = self.entity_factories.read().unwrap();
        factories.get(name).map(|f| f.clone_factory())
    }
    
    pub fn register_field_factory(&self, name: &str, factory: Box<dyn FieldFactory>) {
        let mut factories = self.field_factories.write().unwrap();
        factories.insert(name.to_string(), factory);
    }
    
    pub fn get_field_factory(&self, name: &str) -> Option<Box<dyn FieldFactory>> {
        let factories = self.field_factories.read().unwrap();
        factories.get(name).map(|f| f.clone_factory())
    }
}