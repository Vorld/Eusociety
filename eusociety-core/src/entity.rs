use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity {
    id: u64,
    generation: u64, // For handling ID reuse
}

impl Entity {
    pub fn new(id: u64, generation: u64) -> Self {
        Entity { id, generation }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// EntityManager handles entity creation, deletion and recycling
pub struct EntityManager {
    next_id: u64,
    generations: Vec<u64>,      // Track generations for each entity ID
    recycled_ids: VecDeque<u64>, // Queue of entity IDs that can be recycled
}

impl EntityManager {
    pub fn new() -> Self {
        EntityManager { 
            next_id: 0,
            generations: Vec::new(),
            recycled_ids: VecDeque::new(),
        }
    }

    /// Create a new entity, recycling IDs when possible
    pub fn create(&mut self) -> Entity {
        if let Some(recycled_id) = self.recycled_ids.pop_front() {
            // Use a recycled ID
            let generation = self.generations[recycled_id as usize];
            Entity::new(recycled_id, generation)
        } else {
            // Use a new ID
            let id = self.next_id;
            self.next_id += 1;
            
            // Ensure we have a generation for this ID
            if id as usize >= self.generations.len() {
                self.generations.resize(id as usize + 1, 0);
            }
            
            Entity::new(id, 0)
        }
    }

    /// Mark an entity as deleted, allowing its ID to be recycled
    pub fn delete(&mut self, entity: Entity) {
        let id = entity.id();
        if id as usize >= self.generations.len() {
            return; // Entity ID out of range, ignore
        }
        
        // Increment the generation to invalidate old references
        self.generations[id as usize] += 1;
        
        // Add the ID to the recycled queue
        self.recycled_ids.push_back(id);
    }

    /// Check if an entity is valid (not deleted)
    pub fn is_valid(&self, entity: Entity) -> bool {
        let id = entity.id();
        if id as usize >= self.generations.len() {
            return false;
        }
        
        // Entity is valid if its generation matches the current generation for that ID
        entity.generation() == self.generations[id as usize]
    }
    
    /// Get the current count of active entities
    pub fn entity_count(&self) -> usize {
        self.next_id as usize - self.recycled_ids.len()
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}
