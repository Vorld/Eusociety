use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::broadcast;
use futures_util::{SinkExt, StreamExt};

pub async fn handle_connection(
    stream: TcpStream,
    tx: broadcast::Sender<Vec<u8>>
) {
    let mut rx = tx.subscribe(); // Get a new receiver
    
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };
    
    let (mut ws_sender, _) = ws_stream.split();

    while let Ok(msg) = rx.recv().await {
        if ws_sender.send(Message::Binary(msg)).await.is_err() {
            break;
        }
    }
}
