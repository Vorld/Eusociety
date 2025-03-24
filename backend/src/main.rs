use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_async;
use std::sync::Arc;
use std::path::PathBuf;
use std::fs;
use std::env;
use futures_util::{SinkExt, StreamExt};

// TODO: funny import, there's probably better practices
mod simulation; 
use crate::simulation::{simulation_loop, initialize_registry};
use crate::simulation::config::SimulationConfig;

#[tokio::main]
async fn main() {
    // Initialize registry with default components
    initialize_registry();
    
    // Check for config path from command line
    // BUG: fix useless directory
    let config_path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("EUSOCIETY_CONFIG").ok())
        .unwrap_or_else(|| "config/default.json".to_string());
        
    let config = load_config(&config_path).unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}. Using defaults.", e);
        SimulationConfig::default()
    });
    
    // Log which config we're using
    println!("Using configuration from: {}", config_path);
    
    let addr = "127.0.0.1:3030";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Create a broadcast channel for state updates
    let (tx, _) = broadcast::channel(16);
    let tx_clone = tx.clone();

    // Start simulation in background
    tokio::spawn(async move {
        simulation_loop(tx_clone, config).await;
    });

    // Accept WebSocket connections
    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        tokio::spawn(async move {
            handle_connection(stream, tx).await;
        });
    }
}

async fn handle_connection(stream: TcpStream, tx: broadcast::Sender<Vec<u8>>) {
    let addr = stream.peer_addr().expect("Connected stream should have an address");
    println!("New WebSocket connection: {}", addr);

    let ws_stream = accept_async(stream)
        .await
        .expect("Failed to accept WebSocket connection");

    let (mut ws_sender, _) = ws_stream.split();
    let mut rx = tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if ws_sender.send(Message::Binary(msg)).await.is_err() {
            break;
        }
    }
    
    println!("WebSocket connection closed: {}", addr);
}

fn load_config(path_str: &str) -> Result<SimulationConfig, Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(path_str);
    
    // Try to load the specified config
    let config_data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to read config from {}: {}", path.display(), e);
            
            // If specified config doesn't exist, try default
            if path_str != "config/default.json" {
                println!("Falling back to default config");
                fs::read_to_string("config/default.json")?
            } else {
                return Err(e.into());
            }
        }
    };
    
    // Parse the config JSON
    let config: SimulationConfig = serde_json::from_str(&config_data)?;
    
    Ok(config)
}