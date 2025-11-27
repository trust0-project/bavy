//! WebTransport Relay Server with User-Space Networking
//!
//! A simplified relay server that enables:
//! - Browser to Server connectivity via WebTransport
//! - User-Space NAT Gateway (Slirp) for external network access
//! - Virtual Network Switch behavior (broadcasts traffic between clients)

mod gateway;
// mod stack; // TODO: Integrate smoltcp stack later for full TCP support

use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use wtransport::Identity;
use wtransport::{Endpoint, ServerConfig};

use crate::gateway::{
    generate_arp_reply, generate_icmp_reply, is_arp_request_for_gateway,
    is_external_ipv4_packet, is_icmp_echo_request_to_gateway, is_icmp_packet,
    is_udp_packet, NatGateway,
};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "WebTransport Relay Server for NAT traversal and peer connectivity"
)]
struct Args {
    /// Port to listen on (UDP/QUIC)
    #[arg(short, long, default_value_t = 4433)]
    port: u16,

    /// Bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,
}

/// Run the NAT UDP response receiver loop
async fn run_nat_udp_receiver(
    nat_gateway: Arc<Mutex<NatGateway>>,
    nat_response_tx: broadcast::Sender<Vec<u8>>,
) {
    loop {
        let socket = {
            let nat = nat_gateway.lock().await;
            nat.udp_socket.clone()
        };
        
        if let Some(socket) = socket {
            let mut buf = [0u8; 2048];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((n, src_addr)) => {
                        let frame = {
                            let mut nat = nat_gateway.lock().await;
                            nat.handle_incoming_udp(&buf, src_addr, n)
                        };
                        
                        if let Some(frame) = frame {
                            let _ = nat_response_tx.send(frame);
                        }
                    }
                    Err(_) => break, // Re-acquire socket on error
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let args = Args::parse();

    info!("Starting WebTransport Relay Server (User-Space Mode)...");
    info!("This relay runs without kernel NAT/capabilities.");
    
    // Generate self-signed certificate
    let identity = Identity::self_signed(["localhost", "127.0.0.1", "::1"]).unwrap();
    info!("Certificate Hash (use this in client): {}", identity.certificate_chain().as_slice().first().unwrap().hash());

    // Create virtual switch (broadcast channel)
    // Packets from any client are broadcast to all other clients
    let (switch_tx, _) = broadcast::channel::<Vec<u8>>(1024);

    // Create NAT gateway (User-Space)
    let (nat_response_tx, _) = broadcast::channel::<Vec<u8>>(1024);
    let mut nat_gateway = NatGateway::new(nat_response_tx.clone());
    if let Err(e) = nat_gateway.init().await {
        warn!("Failed to initialize NAT gateway: {}", e);
    }
    let nat_gateway = Arc::new(Mutex::new(nat_gateway));

    // Start NAT UDP receiver
    let nat_gateway_clone = nat_gateway.clone();
    let nat_response_tx_clone = nat_response_tx.clone();
    tokio::spawn(async move {
        run_nat_udp_receiver(nat_gateway_clone, nat_response_tx_clone).await;
    });

    // Bridge NAT responses to the switch
    let switch_tx_nat = switch_tx.clone();
    let mut nat_rx = nat_response_tx.subscribe();
    tokio::spawn(async move {
        loop {
            if let Ok(frame) = nat_rx.recv().await {
                let _ = switch_tx_nat.send(frame);
            }
        }
    });

    // Setup WebTransport server
    let ip: std::net::IpAddr = args.bind.parse()?;
    let socket_addr = SocketAddr::new(ip, args.port);

    let config = ServerConfig::builder()
        .with_bind_address(socket_addr)
        .with_identity(identity)
        .build();

    let endpoint = Endpoint::server(config)?;

    info!("Listening on https://{}:{}", args.bind, args.port);

    // Accept incoming sessions
    loop {
        let incoming_session = endpoint.accept().await;
        
        let switch_tx = switch_tx.clone();
        let nat_gateway = nat_gateway.clone();

        tokio::spawn(async move {
            let request = match incoming_session.await {
                Ok(req) => req,
                Err(e) => {
                    warn!("Connection failed during handshake: {}", e);
                    return;
                }
            };

            info!("New connection from {:?}", request.remote_address());

            let connection = match request.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    warn!("Failed to accept connection: {}", e);
                    return;
                }
            };

            info!("Session established with {:?}", connection.remote_address());

            // Handle the connection
            let mut switch_rx = switch_tx.subscribe();
            
            loop {
                tokio::select! {
                    // Receive from client
                    result = connection.receive_datagram() => {
                        match result {
                            Ok(datagram) => {
                                let data = datagram.to_vec();
                                // Handle Gateway logic locally if applicable
                                let mut handled = false;
                                
                                if is_arp_request_for_gateway(&data) {
                                    let reply = generate_arp_reply(&data);
                                    let _ = connection.send_datagram(reply);
                                    handled = true;
                                } else if is_icmp_echo_request_to_gateway(&data) {
                                    let reply = generate_icmp_reply(&data);
                                    let _ = connection.send_datagram(reply);
                                    handled = true;
                                } else if is_external_ipv4_packet(&data) {
                                    let mut nat = nat_gateway.lock().await;
                                    if is_icmp_packet(&data) {
                                        if nat.process_icmp_outbound(&data).await.is_some() {
                                            handled = true;
                                        }
                                    } else if is_udp_packet(&data) {
                                        if nat.process_udp_outbound(&data).await.is_some() {
                                            handled = true;
                                        }
                                    }
                                }

                                // If not handled by gateway (or even if it was, maybe we broadcast?)
                                // Typically if it's unicast to gateway, we don't broadcast.
                                // If it's broadcast ARP, we broadcast.
                                if !handled {
                                    // Forward to switch (other VMs)
                                    let _ = switch_tx.send(data);
                                }
                            }
                            Err(e) => {
                                info!("Connection closed: {}", e);
                                break;
                            }
                        }
                    }
                    
                    // Send to client (from switch/NAT)
                    Ok(data) = switch_rx.recv() => {
                        if let Err(e) = connection.send_datagram(data) {
                             warn!("Failed to send datagram: {}", e);
                             // break? Or just continue?
                        }
                    }
                }
            }
        });
    }
}
