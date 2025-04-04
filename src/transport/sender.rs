use std::fs::File;
use std::io::{Write, Error as IoError};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{info, error, warn}; // Ensure tracing macros are imported
use std::thread;
use std::time::Duration;

use super::serializer::SerializationError;

/// Error types for transport operations
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    IoError(#[from] IoError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializationError),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Trait for sending serialized data
pub trait Sender: Send + Sync + SenderClone {
    fn send(&self, data: &[u8]) -> Result<(), TransportError>;
    fn flush(&self) -> Result<(), TransportError>;

    /// Try to downcast to WebSocketSender, return None if not a WebSocketSender
    fn as_websocket_sender(&self) -> Option<&WebSocketSender> {
        None
    }
}

impl Clone for Box<dyn Sender> {
    fn clone(&self) -> Self {
        self.clone_sender()
    }
}

/// Helper trait to make Sender cloneable via object-safe methods
pub trait SenderClone {
    fn clone_sender(&self) -> Box<dyn Sender>;
}

// Implement SenderClone for all T that implement Sender + Clone
impl<T> SenderClone for T
where
    T: Sender + Clone + 'static
{
    fn clone_sender(&self) -> Box<dyn Sender> {
        Box::new(self.clone())
    }
}

/// File-based sender implementation
#[derive(Clone)]
pub struct FileSender {
    file_path: String,
    file: Arc<Mutex<File>>,
}

impl FileSender {
    pub fn new(file_path: &str) -> Result<Self, TransportError> {
        let file = File::create(file_path)?;
        Ok(Self {
            file_path: file_path.to_string(),
            file: Arc::new(Mutex::new(file)),
        })
    }
}

impl Sender for FileSender {
    fn send(&self, data: &[u8]) -> Result<(), TransportError> {
        let mut file = self.file.lock().expect("Failed to lock file mutex");
        file.write_all(data)?;
        file.write_all(b"\n")?; // Add a newline for JSON readability
        Ok(())
    }

    fn flush(&self) -> Result<(), TransportError> {
        let mut file = self.file.lock().expect("Failed to lock file mutex");
        file.flush()?;
        Ok(())
    }
}

/// Type alias for a WebSocket client connection
type WebSocketSink = SplitSink<WebSocketStream<TcpStream>, Message>;

/// WebSocket-based sender implementation
#[derive(Clone)]
pub struct WebSocketSender {
    clients: Arc<Mutex<Vec<mpsc::UnboundedSender<Vec<u8>>>>>,
    _runtime: Option<Arc<Runtime>>,
    address: String,
} // Fixed missing closing brace

impl WebSocketSender {
    /// Create a new WebSocket sender that listens on the specified address
    pub fn new(address: &str) -> Result<Self, TransportError> {
        // Create a channel for communication between the WebSocket server and the sender
        let clients = Arc::new(Mutex::new(Vec::new()));
        let clients_clone = Arc::clone(&clients);

        // Try to create a runtime handle - if we're already in a tokio runtime, we'll use that
        // otherwise we'll create our own
        let runtime_handle = Handle::try_current();
        let mut own_runtime = None;

        let runtime_handle = match runtime_handle {
            Ok(handle) => handle,
            Err(_) => {
                // Create a new runtime
                let rt = Runtime::new()
                    .map_err(|e| TransportError::RuntimeError(format!("Failed to create runtime: {}", e)))?;
                let handle = rt.handle().clone();
                own_runtime = Some(Arc::new(rt));
                handle
            }
        };

        let address_clone = address.to_string();

        // Spawn the WebSocket server
        runtime_handle.spawn(async move {
            // Parse the socket address
            let socket_addr: SocketAddr = match address_clone.parse() {
                Ok(addr) => addr,
                Err(err) => {
                    error!("Failed to parse address: {}", err); // Use error!
                    return;
                }
            };

            // Create the TCP listener
            let listener = match TcpListener::bind(&socket_addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    error!("Failed to bind to address: {}", err); // Use error!
                    return;
                }
            };

            info!("WebSocket server listening on: {}", socket_addr); // Use info!

            // Accept connections
            while let Ok((stream, addr)) = listener.accept().await {
                info!("New WebSocket connection from: {}", addr); // Use info!

                let clients = Arc::clone(&clients_clone);

                // Spawn a task to handle the connection
                tokio::spawn(async move {
                    match handle_connection(stream, clients).await {
                        Ok(_) => info!("WebSocket connection to {} closed", addr), // Use info!
                        Err(e) => error!("Error handling WebSocket connection: {}", e), // Use error!
                    }
                });
            }
        });

        // If we created our own runtime, we need to start it
        if let Some(rt) = &own_runtime {
            let rt_handle = rt.handle().clone();
            thread::spawn(move || {
                rt_handle.block_on(async {
                    loop {
                        tokio::time::sleep(Duration::from_secs(3600)).await;
                    }
                });
            });
        }

        Ok(Self {
            clients,
            _runtime: own_runtime,
            address: address.to_string(),
        })
    }

    /// Returns the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.lock().expect("Failed to lock clients mutex").len()
    }
}

impl Sender for WebSocketSender {
    fn send(&self, data: &[u8]) -> Result<(), TransportError> {
        let mut clients = self.clients.lock().expect("Failed to lock clients mutex");

        // Remove clients that encounter errors (they've disconnected)
        clients.retain_mut(|client| {
            match client.send(data.to_vec()) {
                Ok(_) => true,
                Err(_) => false, // Error sending, remove client
            }
        });

        Ok(())
    }

    fn flush(&self) -> Result<(), TransportError> {
        // WebSockets send immediately, no need to flush
        Ok(())
    }

    fn as_websocket_sender(&self) -> Option<&WebSocketSender> {
        Some(self)
    }
}

/// Handle a WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    clients: Arc<Mutex<Vec<mpsc::UnboundedSender<Vec<u8>>>>>,
) -> Result<(), TransportError> {
    // Accept the WebSocket connection
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| TransportError::WebSocketError(format!("Failed to accept WebSocket connection: {}", e)))?;

    // Create a channel for sending data to this client
    let (client_sender, mut client_receiver) = mpsc::unbounded_channel::<Vec<u8>>();

    // Split the WebSocket stream into sender and receiver parts
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Add the client to our list
    clients.lock().expect("Failed to lock clients mutex").push(client_sender);

    // Spawn a task that forwards messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(data) = client_receiver.recv().await {
            if let Err(e) = ws_sender.send(Message::Binary(data)).await {
                error!("Error sending WebSocket message: {}", e); // Use error!
                break;
            }
        }
    });

    // Process incoming WebSocket messages (for completeness)
    let receive_task = tokio::spawn(async move {
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(msg) => {
                    // Handle incoming messages if needed
                    if msg.is_close() {
                        break;
                    }
                },
                Err(e) => {
                    error!("Error receiving WebSocket message: {}", e); // Use error!
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    let _ = tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
    };

    Ok(())
}
