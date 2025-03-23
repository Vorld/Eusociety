pub mod entity;
pub mod engine;
pub mod particle;

use crate::simulation::engine::SimulationEngine;
use crate::simulation::particle::Particle;
use crate::simulation::entity::Entity;
use tokio::sync::broadcast;
use tokio::time::{Instant, Duration, interval};
use rayon::prelude::*;
use std::io::Write;

// Increase broadcast rate to reduce network overhead
const TICK_RATE: Duration = Duration::from_millis(10); // 10 FPS 
const BROADCAST_RATE: u32 = 1; // Send every frame
const INITIAL_PARTICLES: usize = 10000;
const BUFFER_CAPACITY: usize = INITIAL_PARTICLES * 12;
const BATCH_SIZE: usize = 4096;
const MAX_CHUNK_SIZE: usize = 65536; // Define a maximum chunk size for compressed data

pub async fn simulation_loop(tx: broadcast::Sender<Vec<u8>>) {
    let mut engine = SimulationEngine::new(BATCH_SIZE);
    let mut timer = interval(TICK_RATE);
    let mut state_buf = Vec::with_capacity(BUFFER_CAPACITY);
    
    // Initialize particles in parallel
    let particles = (0..INITIAL_PARTICLES)
        .into_par_iter()
        .map(|i| {
            let cols = (INITIAL_PARTICLES as f32).sqrt() as usize;
            let spacing = 6000.0 / cols as f32;
            let x = (i % cols) as f32 * spacing;
            let y = (i / cols) as f32 * spacing;
            Box::new(Particle::new(i, x as f64, y as f64)) as Box<dyn Entity + Send + Sync>
        })
        .collect();

    engine.entities = particles;

    let mut frame_count = 0;
    let mut last_time = Instant::now();
    
    loop {
        timer.tick().await;
        let now = Instant::now();
        let dt = (now - last_time).as_secs_f32();
        last_time = now;

        // Process physics in parallel chunks
        engine.update(dt);

        if frame_count % BROADCAST_RATE == 0 && tx.receiver_count() > 0 {
            engine.serialize_state(&mut state_buf);
            if !state_buf.is_empty() {
            // Broadcast the state directly in chunks if needed
            for chunk in state_buf.chunks(MAX_CHUNK_SIZE) {
                if tx.send(chunk.to_vec()).is_err() {
                break;
                }
            }
            }
            state_buf.clear();
        }
        
        frame_count += 1;
        if frame_count % 60 == 0 {
            println!("FPS: {:.2}, Entities: {}, Connections: {}", 
                1.0 / dt, 
                engine.entities.len(),
                tx.receiver_count()
            );
        }
    }
}