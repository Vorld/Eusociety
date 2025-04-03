pub mod entity;
pub mod engine;
pub mod field;
pub mod transport;
pub mod config;
pub mod registry;

use crate::simulation::engine::SimulationEngine;
use crate::simulation::entity::Entity;
use crate::simulation::config::SimulationConfig;
use crate::simulation::transport::{Transport, Serializer};
use crate::simulation::transport::websocket::{WebSocketTransport, BinarySerializer};
use tokio::sync::broadcast;
use tokio::time::{Instant, Duration, interval};
use std::sync::Arc;

// Initialize the registry with default components
pub fn initialize_registry() {
    use crate::simulation::registry::REGISTRY;
    use crate::simulation::entity::particle::ParticleFactory;
    use crate::simulation::field::scalar_field::ScalarFieldFactory;
    
    REGISTRY.register_entity_factory("particle", Box::new(ParticleFactory));
    REGISTRY.register_field_factory("scalar", Box::new(ScalarFieldFactory));
}

pub async fn simulation_loop(tx: broadcast::Sender<Vec<u8>>, config: SimulationConfig) {
    let mut engine = SimulationEngine::new(config.batch_size).with_config(config.clone());
    let mut timer = interval(Duration::from_millis(config.tick_rate_ms));
    let mut state_buf = Vec::with_capacity(10000 * 13); // Initial capacity
    
    // Initialize the engine with configured entities and fields
    engine.initialize();
    
    // Create transport and serializer
    let serializer = Box::new(BinarySerializer);
    let transport = WebSocketTransport::new(tx.clone(), serializer, config.max_chunk_size);
    
    let mut frame_count = 0;
    let mut last_time = Instant::now();
    
    loop {
        timer.tick().await;
        let now = Instant::now();
        let dt = (now - last_time).as_secs_f32();
        last_time = now;

        // Update simulation
        engine.update(dt);

        // Broadcast state if needed
        if frame_count % config.broadcast_rate == 0 && tx.receiver_count() > 0 {
            state_buf.clear();
            engine.serialize_state(&mut state_buf);
            
            if !state_buf.is_empty() {
                if let Err(e) = transport.send_state(&state_buf) {
                    eprintln!("Error sending state: {}", e);
                }
            }
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