use crate::simulation::entity::{Entity, EntityType, EntityData, EntityFactory};
use crate::simulation::field::{Field, FieldValue};
use crate::simulation::config::{WorldConfig, BoundaryMode};
use serde::{Serialize, Deserialize};
use std::any::Any;
use std::sync::Arc;
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
    pub data: EntityData,
    pub vel_x: f32,
    pub vel_y: f32,
    pub max_speed: f32,
}

impl Particle {
    pub fn new(id: usize, x: f64, y: f64) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            data: EntityData {
                id: id as u32,
                pos_x: x as f32,
                pos_y: y as f32,
                radius: 3.0,
            },
            vel_x: rng.gen_range(-50.0..50.0),
            vel_y: rng.gen_range(-50.0..50.0),
            max_speed: 100.0,
        }
    }
    
    // Enforce boundaries based on the boundary mode
    fn enforce_boundaries(&mut self, world: &WorldConfig) {
        match world.boundary_mode {
            BoundaryMode::Wrap => {
                if self.data.pos_x < 0.0 { self.data.pos_x += world.width; }
                if self.data.pos_x > world.width { self.data.pos_x -= world.width; }
                if self.data.pos_y < 0.0 { self.data.pos_y += world.height; }
                if self.data.pos_y > world.height { self.data.pos_y -= world.height; }
            },
            BoundaryMode::Bounce => {
                if self.data.pos_x < 0.0 { self.data.pos_x = 0.0; self.vel_x = -self.vel_x; }
                if self.data.pos_x > world.width { self.data.pos_x = world.width; self.vel_x = -self.vel_x; }
                if self.data.pos_y < 0.0 { self.data.pos_y = 0.0; self.vel_y = -self.vel_y; }
                if self.data.pos_y > world.height { self.data.pos_y = world.height; self.vel_y = -self.vel_y; }
            },
            // Kill not implemented for particles
            BoundaryMode::Kill => {},
        }
    }
}

impl Entity for Particle {
    fn update(&mut self, dt: f32, world: &WorldConfig, fields: &[Arc<dyn Field>]) {
        // Random jitter
        let mut rng = rand::thread_rng();
        self.vel_x += rng.gen_range(-10.0..10.0);
        self.vel_y += rng.gen_range(-10.0..10.0);
        
        // Apply field effects if any
        for field in fields {
            if field.field_type() == "scalar" {
                if let FieldValue::Scalar(value) = field.get_value(self.data.pos_x, self.data.pos_y) {
                    // Example: Scalar field pushes particles away from high values
                    // This would be customized based on the specific field
                    self.vel_x -= value * 10.0;
                    self.vel_y -= value * 10.0;
                }
            }
        }
        
        // Limit speed
        let speed = (self.vel_x * self.vel_x + self.vel_y * self.vel_y).sqrt();
        if speed > self.max_speed {
            self.vel_x = self.vel_x / speed * self.max_speed;
            self.vel_y = self.vel_y / speed * self.max_speed;
        }
        
        // Update position
        self.data.pos_x += self.vel_x * dt;
        self.data.pos_y += self.vel_y * dt;
        
        // Boundary checks
        self.enforce_boundaries(world);
    }
    
    fn interact_with(&mut self, other: &mut dyn Entity) {
        // Simple collision response
        if other.entity_type() == EntityType::Particle {
            let (other_x, other_y) = other.get_position();
            let dx = self.data.pos_x - other_x;
            let dy = self.data.pos_y - other_y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = self.data.radius + other.get_radius();
            
            if dist_sq < min_dist * min_dist {
                // Very simple elastic collision
                let dist = dist_sq.sqrt();
                let nx = dx / dist;
                let ny = dy / dist;
                
                // Push away
                self.vel_x += nx * 5.0;
                self.vel_y += ny * 5.0;
            }
        }
    }
    
    fn get_position(&self) -> (f32, f32) {
        (self.data.pos_x, self.data.pos_y)
    }
    
    fn get_radius(&self) -> f32 {
        self.data.radius
    }
    
    fn entity_type(&self) -> EntityType {
        EntityType::Particle
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(13);  
        
        // Format: [type:u8, id:u32, x:f32, y:f32]
        buffer.push(EntityType::Particle as u8);
        buffer.extend_from_slice(&self.data.id.to_le_bytes());
        buffer.extend_from_slice(&self.data.pos_x.to_le_bytes());
        buffer.extend_from_slice(&self.data.pos_y.to_le_bytes());
        
        buffer
    }
}

pub struct ParticleFactory;

impl EntityFactory for ParticleFactory {
    fn create_entity(&self, id: u32, x: f32, y: f32, properties: &serde_json::Value) -> Box<dyn Entity> {
        // Parse properties if any
        let max_speed = properties.get("max_speed")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(100.0);
            
        let mut particle = Particle::new(id as usize, x as f64, y as f64);
        particle.max_speed = max_speed;
        
        Box::new(particle)
    }
    
    fn entity_type(&self) -> EntityType {
        EntityType::Particle
    }

    fn clone_factory(&self) -> Box<dyn EntityFactory> {
        Box::new(ParticleFactory)
    }
}