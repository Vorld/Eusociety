use rayon::prelude::*;
use crate::simulation::entity::Entity;

pub struct SimulationEngine {
    pub entities: Vec<Box<dyn Entity + Send + Sync>>,
    chunk_size: usize,
}

impl SimulationEngine {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            entities: Vec::new(),
            chunk_size,
        }
    }

    pub fn add_entity(&mut self, entity: Box<dyn Entity + Send + Sync>) {
        self.entities.push(entity);
    }

    pub fn update(&mut self, dt: f32) {
        // Process chunks in parallel
        self.entities.par_chunks_mut(self.chunk_size).for_each(|chunk| {
            for entity in chunk {
                entity.update(dt);
            }
        });
    }

    pub fn serialize_state(&self, buf: &mut Vec<u8>) {
        // Pre-allocate space to avoid reallocations
        let mut local_buffers: Vec<_> = self.entities
            .par_chunks(self.chunk_size)
            .map(|chunk| {
                let mut local_buf = Vec::with_capacity(chunk.len() * 12);
                for entity in chunk {
                    local_buf.extend(entity.serialize());
                }
                local_buf
            })
            .collect();

        // Concatenate results
        buf.clear();
        buf.reserve(self.entities.len() * 12);
        for local_buf in local_buffers {
            buf.extend(local_buf);
        }
    }
}
