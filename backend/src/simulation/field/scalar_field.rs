use crate::simulation::field::{Field, FieldValue, FieldFactory};
use serde::{Serialize, Deserialize};
use std::any::Any;

#[derive(Debug)]
pub struct ScalarField {
    width: f32,
    height: f32,
    resolution: usize,
    cell_width: f32,
    cell_height: f32,
    values: Vec<f32>,
    decay_rate: f32,
    diffusion_rate: f32,
}

impl ScalarField {
    pub fn new(width: f32, height: f32, resolution: usize, decay_rate: f32, diffusion_rate: f32) -> Self {
        let cell_width = width / resolution as f32;
        let cell_height = height / resolution as f32;
        let values = vec![0.0; resolution * resolution];
        
        Self {
            width,
            height,
            resolution,
            cell_width,
            cell_height,
            values,
            decay_rate,
            diffusion_rate,
        }
    }
    
    fn cell_index(&self, x: f32, y: f32) -> usize {
        let x_idx = (x / self.cell_width) as usize;
        let y_idx = (y / self.cell_height) as usize;
        let x_idx = x_idx.min(self.resolution - 1);
        let y_idx = y_idx.min(self.resolution - 1);
        y_idx * self.resolution + x_idx
    }
}

impl Field for ScalarField {
    fn get_value(&self, x: f32, y: f32) -> FieldValue {
        let idx = self.cell_index(x, y);
        FieldValue::Scalar(self.values[idx])
    }
    
    fn add_value(&mut self, x: f32, y: f32, value: FieldValue) {
        if let FieldValue::Scalar(val) = value {
            let idx = self.cell_index(x, y);
            self.values[idx] += val;
        }
    }
    
    fn update(&mut self, dt: f32) {
        // Apply decay
        if self.decay_rate > 0.0 {
            for val in &mut self.values {
                *val *= (1.0 - self.decay_rate * dt).max(0.0);
            }
        }
        
        // Apply diffusion
        if self.diffusion_rate > 0.0 {
            let mut new_values = self.values.clone();
            
            for y in 0..self.resolution {
                for x in 0..self.resolution {
                    let idx = y * self.resolution + x;
                    let current = self.values[idx];
                    
                    // Get neighboring cells with wrapping
                    let left = if x > 0 { 
                        self.values[y * self.resolution + (x - 1)] 
                    } else { 
                        self.values[y * self.resolution + (self.resolution - 1)] 
                    };
                    
                    let right = if x < self.resolution - 1 { 
                        self.values[y * self.resolution + (x + 1)] 
                    } else { 
                        self.values[y * self.resolution] 
                    };
                    
                    let up = if y > 0 { 
                        self.values[(y - 1) * self.resolution + x] 
                    } else { 
                        self.values[(self.resolution - 1) * self.resolution + x] 
                    };
                    
                    let down = if y < self.resolution - 1 { 
                        self.values[(y + 1) * self.resolution + x] 
                    } else { 
                        self.values[x] 
                    };
                    
                    // Calculate diffusion
                    let diffusion = (left + right + up + down - 4.0 * current) * self.diffusion_rate * dt;
                    new_values[idx] += diffusion;
                }
            }
            
            self.values = new_values;
        }
    }
    
    fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self.values).unwrap_or_default()
    }
    
    fn field_type(&self) -> &'static str {
        "scalar"
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct ScalarFieldFactory;

impl FieldFactory for ScalarFieldFactory {
    fn create_field(&self, width: f32, height: f32, resolution: usize, 
                    properties: &serde_json::Value) -> Box<dyn Field> {
        let decay_rate = properties.get("decay_rate")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.1);
            
        let diffusion_rate = properties.get("diffusion_rate")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.05);
            
        Box::new(ScalarField::new(width, height, resolution, decay_rate, diffusion_rate))
    }
    
    fn field_type(&self) -> &'static str {
        "scalar"
    }

    fn clone_factory(&self) -> Box<dyn FieldFactory> {
        Box::new(ScalarFieldFactory)
    }
}