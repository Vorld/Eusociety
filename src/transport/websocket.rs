//! Implements the WebSocket `Sender` for transmitting simulation data over network connections.
//!
//! This module sets up a Tokio-based asynchronous WebSocket server that listens for
//! incoming client connections. It manages connected clients and broadcasts serialized
//! simulation state data to all of them.

use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{info, error};
use std::thread;
use std::time::Duration;

// Use super to access items from the parent module (transport)
use super::{TransportError, Sender, SenderClone};

/// Sender implementation that broadcasts data to connected WebSocket clients.
///
/// Sets up an asynchronous WebSocket server using Tokio and `tokio-tungstenite`.
/// Manages client connections and uses unbounded channels (`mpsc`) to distribute
/// data efficiently. Can optionally create and manage its own Tokio runtime if
/// not already running within one.
#[derive(Clone)]
pub struct WebSocketSender {
    /// Shared, thread-safe list of sender channels, one for each connected client.
    clients: Arc<Mutex<Vec<mpsc::UnboundedSender<Arc<Vec<u8>>>>>>, 
    /// Holds the Tokio runtime if this sender created it. `None` if running inside an existing runtime.
    _runtime: Option<Arc<Runtime>>, 
    /// The network address the server is configured to listen on.
    _address: String, 
}

impl WebSocketSender {
    /// Creates a new `WebSocketSender` and starts the server listening on the specified address.
    ///
    /// Spawns the server logic onto an existing Tokio runtime if available, otherwise
    /// creates a new runtime and runs it in a background thread.
    ///
    /// # Arguments
    ///
    /// * `address` - The network address string (e.g., "127.0.0.1:9001") to bind the server to.
    ///
    /// # Errors
    ///
    /// Returns `TransportError` if:
    /// - A new Tokio runtime cannot be created (if needed).
    /// - The provided address cannot be parsed.
    /// - The server fails to bind to the specified address.
    pub fn new(address: &str) -> Result<Self, TransportError> {
        info!("Initializing WebSocketSender for address: {}", address);
        let clients = Arc::new(Mutex::new(Vec::new()));
        let clients_clone = Arc::clone(&clients);

        // Try to get the current Tokio runtime handle or create a new one
        let runtime_handle = Handle::try_current();
        let mut own_runtime = None;

        let runtime_handle = match runtime_handle {
            Ok(handle) => handle,
            Err(_) => {
                // Create a new runtime if not already in one
                let rt = Runtime::new()
                    .map_err(|e| TransportError::RuntimeError(format!("Failed to create runtime: {}", e)))?;
                let handle = rt.handle().clone();
                own_runtime = Some(Arc::new(rt)); // Store the runtime Arc
                handle
            }
        };

        let address_clone = address.to_string();

        // Spawn the WebSocket server task onto the runtime
        runtime_handle.spawn(async move {
            let socket_addr: SocketAddr = match address_clone.parse() {
                Ok(addr) => addr,
                Err(err) => {
                    error!("Failed to parse WebSocket address '{}': {}", address_clone, err);
                    return;
                }
            };

            let listener = match TcpListener::bind(&socket_addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    error!("Failed to bind WebSocket listener to {}: {}", socket_addr, err);
                    return;
                }
            };

            info!("WebSocket server listening on: {}", socket_addr);

            // Accept incoming connections loop
            while let Ok((stream, addr)) = listener.accept().await {
                info!("New WebSocket connection from: {}", addr);
                let clients_for_handler = Arc::clone(&clients_clone);
                // Spawn a task for each connection
                tokio::spawn(async move {
                    match handle_connection(stream, clients_for_handler).await {
                        Ok(_) => info!("WebSocket connection to {} closed gracefully", addr),
                        Err(e) => error!("Error handling WebSocket connection from {}: {}", addr, e),
                    }
                });
            }
        });

        // If we created our own runtime, keep it alive in a background thread
        // This is a simple way; more robust solutions might involve managing the runtime lifecycle better.
        if let Some(rt_arc) = &own_runtime {
            let rt_handle_clone = rt_arc.handle().clone();
             thread::spawn(move || {
                 info!("Starting background thread for owned Tokio runtime.");
                 // Block on an empty future to keep the runtime alive
                 rt_handle_clone.block_on(async {
                     loop {
                         tokio::time::sleep(Duration::from_secs(60)).await; // Keep thread alive
                     }
                 });
                 info!("Background thread for owned Tokio runtime finished."); // Should not happen in normal operation
             });
        }


        Ok(Self {
            clients,
            _runtime: own_runtime,
            _address: address.to_string(),
        })
    }

    /// Returns the number of currently connected WebSocket clients.
    pub fn client_count(&self) -> usize {
        match self.clients.lock() {
            Ok(guard) => guard.len(), // Get length of the client list
            Err(e) => {
                error!("Failed to lock clients mutex for counting: {}", e);
                0 // Return 0 if mutex is poisoned
            }
        }
    }
}

impl SenderClone for WebSocketSender {
    /// Clones the `WebSocketSender`. This is a shallow clone due to `Arc`.
    fn clone_sender(&self) -> Box<dyn Sender> {
        Box::new(self.clone())
    }
}

impl Sender for WebSocketSender {
    /// Sends the provided data as a binary WebSocket message to all connected clients.
    ///
    /// Clones the data into an `Arc` for efficient sharing across multiple client send tasks.
    /// Removes clients from the list if sending to them fails (indicating disconnection).
    fn send(&self, data: &[u8]) -> Result<(), TransportError> {
        if data.is_empty() { 
            // tracing::trace!("WebSocketSender::send called with empty data, skipping.");
            return Ok(()); 
        } 

        // Wrap data in Arc for cheap cloning per client
        let data_arc = Arc::new(data.to_vec());
        
        // Lock the client list mutex
        let mut clients_guard = self.clients.lock().map_err(|e| TransportError::RuntimeError(format!("Client mutex poisoned in send: {}", e)))?;

        // Iterate and send, removing clients that error out
        clients_guard.retain_mut(|client_tx| {
            match client_tx.send(Arc::clone(&data_arc)) {
                Ok(_) => true, // Keep client if send succeeds
                Err(_) => {
                    // Error likely means client disconnected
                    info!("WebSocket client disconnected (send error), removing.");
                    false // Remove client by returning false from retain_mut
                }
            }
        });

        Ok(())
    }

    /// Flushing is typically a no-op for WebSockets as sends are usually immediate.
    fn flush(&self) -> Result<(), TransportError> {
        // tracing::trace!("WebSocketSender flush called (no-op).");
        Ok(())
    }

    /// Overrides the default `Sender::as_websocket_sender` to return `Some(self)`.
    fn as_websocket_sender(&self) -> Option<&WebSocketSender> {
        Some(self) // This implementation *is* a WebSocketSender
    }
}

/// Asynchronous function to handle a single accepted WebSocket connection.
///
/// This function performs the WebSocket handshake, sets up channels for communication,
/// and spawns tasks to handle sending data *to* the client and receiving data *from* the client.
/// It also ensures the client is removed from the shared list upon disconnection.
///
/// # Arguments
///
/// * `stream` - The raw TCP stream for the accepted connection.
/// * `clients` - The shared list of client sender channels.
async fn handle_connection(
    stream: TcpStream,
    clients: Arc<Mutex<Vec<mpsc::UnboundedSender<Arc<Vec<u8>>>>>>, 
) -> Result<(), TransportError> {
    // Perform the WebSocket handshake
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| TransportError::WebSocketError(format!("WebSocket handshake failed: {}", e)))?;
    
    info!("WebSocket handshake successful.");

    // Create an unbounded channel for this specific client.
    // The main send loop will put messages into the sender part.
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Arc<Vec<u8>>>();

    // Add the sender part of the channel to the shared client list.
    // Keep a clone (`client_id`) to identify this client for removal later.
    let client_id = client_tx.clone(); 
    clients.lock().map_err(|e| TransportError::RuntimeError(format!("Client mutex poisoned on add: {}", e)))?.push(client_tx);
    
    // Split the WebSocket stream into a sender (sink) and receiver (stream).
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // --- Spawn Send Task ---
    // This task listens on the client's channel (`client_rx`) and forwards messages
    // to the client's WebSocket sink (`ws_sink`).
    let send_task = tokio::spawn(async move {
        while let Some(data_arc) = client_rx.recv().await { 
            // Convert Arc<Vec<u8>> back to Vec<u8> for the Message::Binary variant
            let data_vec = data_arc.as_ref().clone(); 
            if ws_sink.send(Message::Binary(data_vec)).await.is_err() {
                // Error sending probably means the client disconnected.
                info!("Send task: Error sending to WebSocket sink, client likely disconnected.");
                break; 
            }
        }
        // Attempt to close the sink gracefully when the channel is closed or sending fails.
        info!("Send task: Channel closed or send error, closing WebSocket sink.");
        let _ = ws_sink.close().await; 
    });

    // --- Spawn Receive Task ---
    // This task listens for incoming messages from the client's WebSocket stream (`ws_stream`).
    // It currently only handles close messages but could be extended (e.g., for pings).
    let receive_task = tokio::spawn(async move {
        while let Some(message) = ws_stream.next().await {
            match message {
                Ok(msg) => {
                    // tracing::trace!("Received WebSocket message: {:?}", msg);
                    if msg.is_close() {
                        info!("Receive task: Received close frame from client.");
                        break; 
                    }
                    // TODO: Handle ping/pong or other message types if necessary
                },
                Err(e) => {
                    // Error receiving probably means the client disconnected abruptly.
                    info!("Receive task: Error receiving from WebSocket stream: {}", e);
                    break; 
                }
            }
        }
        info!("Receive task: Exiting loop.");
    });

    // Keep the connection alive until either the send or receive task finishes.
    tokio::select! {
        _ = send_task => info!("Send task finished."),
        _ = receive_task => info!("Receive task finished."),
    };

    // --- Cleanup ---
    // Remove this client's sender channel from the shared list.
    info!("Removing disconnected client from list.");
    clients.lock().map_err(|e| TransportError::RuntimeError(format!("Client mutex poisoned on remove: {}", e)))?
           .retain(|sender| !sender.same_channel(&client_id)); // Use same_channel for reliable comparison
    info!("Client removed.");

    Ok(())
}
