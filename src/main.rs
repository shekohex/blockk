#![deny(
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces,
    unused_results,
    warnings,
    missing_copy_implementations,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates
)]
use std::net::{SocketAddr, ToSocketAddrs};

use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

mod block_ads;
mod request_handler;
mod tunnel;

use request_handler::*;
use tunnel::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // We create a tokio tcp listener on the localhost:5555 (by default) address
    // but read the port from the environment variable PORT
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| String::from("5555"))
        .parse::<u16>()
        .unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    // now we can accept connections in a loop
    // and for each connection we spawn a new task
    // that will handle the connection
    println!("Listening on {}", addr);
    // now we setup the logger
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::fmt()
        .with_target(true)
        .without_time()
        .with_env_filter(env_filter)
        .init();
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::info!("Connection from: {}", peer_addr);
        let _ = tokio::spawn(async move {
            match handle_connection(stream).await {
                Err(e) => {
                    tracing::error!("Error: {:?}", e);
                }
                Ok(ConnectionResult::InvalidRequest(method)) => {
                    tracing::error!("{} is not support", method);
                }
                Ok(ConnectionResult::Connect(ConnectResult::InvalidDestination(dest))) => {
                    tracing::warn!("{} is not a allowed destination", dest);
                }
                Ok(ConnectionResult::Connect(ConnectResult::Success(client, dest, stats))) => {
                    assert_eq!(peer_addr, client);
                    tracing::info!(
                        "{} <-> Proxy <-> {}:\n\t{} -> {}: {} bytes\n\t{} -> {}: {} bytes",
                        client,
                        dest,
                        client,
                        dest,
                        stats.client_to_dest,
                        dest,
                        client,
                        stats.dest_to_client
                    );
                }
            }
            tracing::info!("Connection from {} is ended", peer_addr);
        });
    }
}

// The result of the HTTP request
enum ConnectionResult {
    InvalidRequest(String),
    Connect(ConnectResult),
}

// The result of the HTTP CONNECT request
enum ConnectResult {
    InvalidDestination(String),
    Success(SocketAddr, SocketAddr, TunnelStats),
}

async fn handle_connection(mut tcp_stream: TcpStream) -> anyhow::Result<ConnectionResult> {
    let client_addr = tcp_stream.peer_addr()?;
    let req = get_request(&mut tcp_stream).await?;
    match req.method.name.as_str() {
        "CONNECT" => {
            let r = handle_connect_request(tcp_stream, client_addr, req.method.uri).await?;
            Ok(ConnectionResult::Connect(r))
        }
        method => {
            end_invalid_request(tcp_stream, ServerResponse::MethodNotAllowed).await?;
            Ok(ConnectionResult::InvalidRequest(method.to_string()))
        }
    }
}

async fn handle_connect_request(
    client: TcpStream,
    client_addr: SocketAddr,
    destination_uri: String,
) -> anyhow::Result<ConnectResult> {
    match destination_uri.to_socket_addrs()?.next() {
        None => {
            end_invalid_request(client, ServerResponse::BadRequest).await?;
            Ok(ConnectResult::InvalidDestination(destination_uri))
        }
        Some(dest_addr) => {
            tracing::debug!("Opening {}", destination_uri);
            // check if we should block this destination uri
            let should_block = block_ads::should_block(&destination_uri);
            if should_block {
                end_invalid_request(client, ServerResponse::Forbidden).await?;
                tracing::warn!("Blocked {}", destination_uri);
                return Ok(ConnectResult::InvalidDestination(destination_uri));
            }
            let stats = process_connect_request(client, client_addr, dest_addr).await?;
            Ok(ConnectResult::Success(client_addr, dest_addr, stats))
        }
    }
}

async fn process_connect_request(
    mut client: TcpStream,
    client_addr: SocketAddr,
    dest_addr: SocketAddr,
) -> anyhow::Result<TunnelStats> {
    let client_name = format!("{}", client_addr);
    let dest_name = format!("{}", dest_addr);
    let dest = TcpStream::connect(dest_addr).await?;
    send_response(&mut client, ServerResponse::Ok).await?;
    let mut tunnel = Tunnel::new(client_name, client, dest_name, dest);
    Ok(tunnel.start().await?)
}

async fn end_invalid_request(mut client: TcpStream, res: ServerResponse) -> anyhow::Result<()> {
    send_response(&mut client, res).await?;
    client.shutdown().await?;
    Ok(())
}
