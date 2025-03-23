mod simulation;
mod ws_server;

use tokio::net::TcpListener;
use simulation::simulation_loop;
use ws_server::handle_connection;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<Vec<u8>>(16);

    let simulation_tx = tx.clone();
    tokio::spawn(async move {
        simulation_loop(simulation_tx).await;
    });

    let addr = "127.0.0.1:3030";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("Backend server is listening on ws://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        tokio::spawn(handle_connection(stream, tx));
    }
}
