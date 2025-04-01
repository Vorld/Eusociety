use eusociety_core::World;
use eusociety_simulation::Position; // Use the actual Position from simulation crate
use serde::Serialize;
use std::io::{self, Write};

// --- Error Type ---
#[derive(Debug)]
pub enum TransportError {
    Serialization(serde_json::Error),
    Io(io::Error),
    #[cfg(feature = "websocket")]
    WebSocket(String),
    // Add more specific errors later
}

impl From<serde_json::Error> for TransportError {
    fn from(err: serde_json::Error) -> Self { TransportError::Serialization(err) }
}

impl From<io::Error> for TransportError {
    fn from(err: io::Error) -> Self { TransportError::Io(err) }
}


// --- Traits ---
/// Serializes relevant world state into a byte representation.
pub trait Serializer: Send + Sync {
    fn serialize(&self, world: &World) -> Result<String, TransportError>;
}

/// Sends serialized data to a destination.
pub trait Sender {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
}

// --- M1 Implementations ---

#[derive(Serialize, Debug)]
struct EntityPosition {
    id: u64,
    generation: u64,
    x: f32,
    y: f32,
}

#[derive(Serialize, Debug)]
struct WorldState {
    entities: Vec<EntityPosition>,
    time: f32, // Add simulation time
}

/// Serializes entities with Position components to JSON for M1.
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn serialize(&self, world: &World) -> Result<String, TransportError> {
        // Query all entities with Position components
        let positions = world.query::<Position>();
        
        // Map to our serializable structure
        let entities: Vec<EntityPosition> = positions
            .iter()
            .map(|(entity, pos)| {
                EntityPosition {
                    id: entity.id(),
                    generation: entity.generation(),
                    x: pos.x,
                    y: pos.y,
                }
            })
            .collect();
        
        // Get the current simulation time if available
        let time = world.get_resource::<eusociety_simulation::DeltaTime>()
            .map_or(0.0, |dt| dt.0);
            
        let world_state = WorldState { 
            entities,
            time,
        };
        
        let json_string = serde_json::to_string(&world_state)?;
        Ok(json_string)
    }
}

/// Sends data to standard output for M1.
pub struct StdioSender {
    stdout: io::Stdout,
}

impl StdioSender {
    pub fn new() -> Self {
        StdioSender { stdout: io::stdout() }
    }
}

impl Sender for StdioSender {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stdout.write_all(data)?;
        self.stdout.write_all(b"\n")?; // Add newline for readability
        self.stdout.flush()?; // Ensure it's written immediately
        Ok(())
    }
}

impl Default for StdioSender {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializes entities with Position components to a binary format for improved performance.
/// Uses a simple binary format:
/// - 4 bytes: entity count as u32
/// - For each entity:
///   - 8 bytes: entity ID as u64
///   - 8 bytes: generation as u64
///   - 4 bytes: x position as f32
///   - 4 bytes: y position as f32
pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    fn serialize(&self, world: &World) -> Result<String, TransportError> {
        let positions = world.query::<Position>();
        
        // Calculate the size needed for the binary data
        let entity_count = positions.len();
        let buffer_size = 4 + (entity_count * (8 + 8 + 4 + 4));
        let mut buffer = Vec::with_capacity(buffer_size);
        
        // Write entity count
        buffer.extend_from_slice(&(entity_count as u32).to_le_bytes());
        
        // Write each entity's data
        for (entity, position) in &positions {
            // Entity ID
            buffer.extend_from_slice(&entity.id().to_le_bytes());
            // Generation
            buffer.extend_from_slice(&entity.generation().to_le_bytes());
            // Position X
            buffer.extend_from_slice(&position.x.to_le_bytes());
            // Position Y
            buffer.extend_from_slice(&position.y.to_le_bytes());
        }
        
        // Convert to base64 string for easier transmission
        let base64 = base64::encode(&buffer);
        Ok(base64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Need to mock World or create a test World setup later
    // For now, test the basic structure

    #[test]
    fn json_serializer_placeholder() {
        // TODO: Create a mock World when core is more developed
        let world = World::new(); // Assuming World::new() exists
        let serializer = JsonSerializer;
        let result = serializer.serialize(&world);
        assert!(result.is_ok());
        // Basic check on placeholder output structure
        assert!(result.unwrap().contains(r#""entities":[{"id":"#));
    }

    // Testing StdioSender is tricky in unit tests, better suited for integration tests.
}

#[cfg(feature = "websocket")]
mod websocket {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::runtime::Runtime;
    use tokio_tungstenite::tungstenite::{Message, Error as WsError};
    use tokio_tungstenite::accept_async;
    use tokio::sync::broadcast;
    use std::collections::HashSet;
    use futures::stream::StreamExt;
    use futures::sink::SinkExt;

    /// WebSocket sender that broadcasts world state to connected clients
    pub struct WebSocketSender {
        host: String,
        port: u16,
        tx: Option<broadcast::Sender<String>>,
        runtime: Option<Runtime>,
        clients_count: Arc<Mutex<usize>>,
        active_clients: Arc<Mutex<HashSet<String>>>,
    }

    impl WebSocketSender {
        pub fn new(host: &str, port: u16) -> Self {
            WebSocketSender {
                host: host.to_string(),
                port,
                tx: None,
                runtime: None,
                clients_count: Arc::new(Mutex::new(0)),
                active_clients: Arc::new(Mutex::new(HashSet::new())),
            }
        }
        
        /// Starts the WebSocket server in a background thread
        pub fn start(&mut self) -> Result<(), TransportError> {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| TransportError::WebSocket(e.to_string()))?;
            
            let addr = format!("{}:{}", self.host, self.port);
            let socket_addr: SocketAddr = addr.parse()
                .map_err(|e| TransportError::WebSocket(format!("Invalid address: {}", e)))?;
            
            // Create a broadcast channel for messages
            let (tx, _) = broadcast::channel::<String>(16);
            self.tx = Some(tx.clone());
            
            let clients_count = self.clients_count.clone();
            let active_clients = self.active_clients.clone();
            
            // Spawn the WebSocket server
            let _server_handle = runtime.spawn(async move {
                let listener = TcpListener::bind(&socket_addr).await
                    .expect("Failed to bind WebSocket server");
                
                println!("WebSocket server listening on: {}", socket_addr);
                
                while let Ok((stream, addr)) = listener.accept().await {
                    let peer = addr.to_string();
                    
                    // Check if this client is already connected
                    {
                        let mut clients = active_clients.lock().unwrap();
                        if clients.contains(&peer) {
                            continue;
                        }
                        
                        // Add to active clients
                        clients.insert(peer.clone());
                    }
                    
                    // Update client count
                    {
                        let mut count = clients_count.lock().unwrap();
                        *count += 1;
                        println!("Client connected: {}. Total clients: {}", peer, *count);
                    }
                    
                    // Create a new receiver for this client
                    let rx = tx.subscribe();
                    
                    // Handle the connection in a new task
                    tokio::spawn(handle_connection(
                        stream, 
                        rx, 
                        peer.clone(), 
                        clients_count.clone(), 
                        active_clients.clone()
                    ));
                }
            });
            
            self.runtime = Some(runtime);
            
            Ok(())
        }
        
        /// Returns the number of connected clients
        pub fn client_count(&self) -> usize {
            *self.clients_count.lock().unwrap()
        }
        
        /// Check if a specific client is connected
        fn is_client_connected(&self, peer: &str) -> bool {
            let clients = self.active_clients.lock().unwrap();
            clients.contains(peer)
        }
    }
    
    impl Sender for WebSocketSender {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            if let Some(tx) = &self.tx {
                let data_str = std::str::from_utf8(data)
                    .map_err(|e| TransportError::WebSocket(format!("Invalid UTF-8: {}", e)))?;
                
                let client_count = self.client_count();
                if client_count > 0 {
                    // Only log broadcast errors if they're not related to lack of receivers
                    match tx.send(data_str.to_string()) {
                        Ok(_) => {},
                        Err(e) => {
                            // Check if there are no receivers - this is normal when all clients disconnect
                            if e.to_string().contains("no receivers") {
                                // All receivers are gone, this is normal if all clients disconnected
                                // No need to report an error
                            } else {
                                // Other broadcast errors should still be reported
                                return Err(TransportError::WebSocket(format!("Broadcast error: {}", e)));
                            }
                        }
                    }
                }
                Ok(())
            } else {
                Err(TransportError::WebSocket("WebSocket server not started".to_string()))
            }
        }
    }
    
    async fn handle_connection(
        raw_stream: TcpStream, 
        mut rx: broadcast::Receiver<String>,
        peer: String,
        clients_count: Arc<Mutex<usize>>,
        active_clients: Arc<Mutex<HashSet<String>>>
    ) {
        // Accept the WebSocket connection
        let ws_stream = match accept_async(raw_stream).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Error during WebSocket handshake: {}", e);
                // Remove client from active list
                remove_client(&peer, &clients_count, &active_clients);
                return;
            }
        };
        
        // Listen to the broadcast channel and forward messages to the client
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Create a task to listen for client disconnections
        let peer_clone = peer.clone();
        let clients_count_clone = clients_count.clone();
        let active_clients_clone = active_clients.clone();
        
        let receive_task = tokio::spawn(async move {
            while let Some(msg_result) = ws_receiver.next().await {
                match msg_result {
                    Ok(_) => {
                        // Client sent a message, which we currently ignore
                        // Could process client -> server messages here if needed
                    },
                    Err(e) => {
                        // Connection error
                        if !matches!(e, WsError::ConnectionClosed | WsError::Protocol(_) | WsError::Utf8) {
                            eprintln!("WebSocket receive error: {} - {}", peer_clone, e);
                        }
                        break;
                    }
                }
            }
            
            // Client disconnected
            remove_client(&peer_clone, &clients_count_clone, &active_clients_clone);
        });
        
        // Send messages from broadcast to client
        let _send_result = async {
            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Ok(data) => {
                                // Check if client is still in active list before sending
                                {
                                    let clients = active_clients.lock().unwrap();
                                    if !clients.contains(&peer) {
                                        break;
                                    }
                                }
                                
                                // Send with proper error handling for broken connections
                                match ws_sender.send(Message::Text(data)).await {
                                    Ok(_) => {},
                                    Err(e) => {
                                        // Only log non-disconnect errors
                                        if !is_disconnect_error(&e) {
                                            eprintln!("WebSocket send error: {} - {}", peer, e);
                                        }
                                        break;
                                    }
                                }
                            },
                            Err(_) => break,
                        }
                    }
                }
            }
            
            // When send loop breaks, ensure client is removed
            remove_client(&peer, &clients_count, &active_clients);
        }.await;
        
        // Make sure client is removed on task completion
        let _ = receive_task.await;
    }
    
    // Helper function to check if an error is due to disconnection
    fn is_disconnect_error(e: &WsError) -> bool {
        match e {
            WsError::ConnectionClosed => true,
            WsError::AlreadyClosed => true,
            WsError::Io(io_err) if io_err.kind() == std::io::ErrorKind::BrokenPipe => true,
            WsError::Io(io_err) if io_err.kind() == std::io::ErrorKind::ConnectionReset => true,
            WsError::Io(io_err) if io_err.kind() == std::io::ErrorKind::ConnectionAborted => true,
            _ => false,
        }
    }
    
    // Helper function to remove a client from tracking structures
    fn remove_client(
        peer: &str,
        clients_count: &Arc<Mutex<usize>>,
        active_clients: &Arc<Mutex<HashSet<String>>>
    ) {
        let mut was_active = false;
        
        // Remove from active clients set
        {
            let mut clients = active_clients.lock().unwrap();
            was_active = clients.remove(peer);
        }
        
        // Only decrement count if client was active
        if was_active {
            let mut count = clients_count.lock().unwrap();
            // Guard against underflow
            if *count > 0 {
                *count -= 1;
            }
            println!("Client disconnected: {}. Total clients: {}", peer, *count);
        }
    }
}

// Re-export WebSocketSender when websocket feature is enabled
#[cfg(feature = "websocket")]
pub use websocket::WebSocketSender;
