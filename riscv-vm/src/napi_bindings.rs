//! Node.js native addon bindings via napi-rs.
//!
//! This module exposes WebTransport client functionality to Node.js,
//! reusing the existing native WebTransport implementation.

use napi_derive::napi;
use napi_rs::bindgen_prelude::*;

// Re-export napi for the macro to find
use napi_rs as napi;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use wtransport::ClientConfig;
use wtransport::Endpoint;
use wtransport::tls::Sha256Digest;

/// Message type prefix for control messages
const MSG_TYPE_CONTROL: u8 = 0x00;
/// Message type prefix for Ethernet data frames  
const MSG_TYPE_DATA: u8 = 0x01;
/// Message type prefix for chunked frames
const MSG_TYPE_CHUNKED: u8 = 0x02;

/// Maximum payload size for a single chunk (leaving room for header + overhead)
const MAX_CHUNK_PAYLOAD: usize = 900;

/// Threshold above which we switch to chunking
const CHUNK_THRESHOLD: usize = 950;

#[derive(Debug, Clone)]
struct ChunkInfo {
    chunk_id: u16,
    chunk_index: u8,
    total_chunks: u8,
    payload: Vec<u8>,
}

fn encode_chunked_frame(ethernet_frame: &[u8], chunk_id: u16) -> Vec<Vec<u8>> {
    let total_len = ethernet_frame.len();
    let total_chunks = (total_len + MAX_CHUNK_PAYLOAD - 1) / MAX_CHUNK_PAYLOAD;
    let mut messages = Vec::with_capacity(total_chunks);

    for (i, chunk_data) in ethernet_frame.chunks(MAX_CHUNK_PAYLOAD).enumerate() {
        let mut frame = Vec::with_capacity(5 + chunk_data.len());
        frame.push(MSG_TYPE_CHUNKED);
        frame.extend(&chunk_id.to_be_bytes()); // 2 bytes
        frame.push(i as u8);
        frame.push(total_chunks as u8);
        frame.extend(chunk_data);
        messages.push(frame);
    }

    messages
}

fn decode_chunk(data: &[u8]) -> Option<ChunkInfo> {
    if data.len() < 5 || data[0] != MSG_TYPE_CHUNKED {
        return None;
    }
    
    let chunk_id = u16::from_be_bytes([data[1], data[2]]);
    let chunk_index = data[3];
    let total_chunks = data[4];
    let payload = data[5..].to_vec();

    Some(ChunkInfo {
        chunk_id,
        chunk_index,
        total_chunks,
        payload,
    })
}

fn encode_frame_smart(ethernet_frame: &[u8], chunk_id_counter: &mut u16) -> Vec<Vec<u8>> {
    if ethernet_frame.len() > CHUNK_THRESHOLD {
        let chunk_id = *chunk_id_counter;
        *chunk_id_counter = chunk_id_counter.wrapping_add(1);
        encode_chunked_frame(ethernet_frame, chunk_id)
    } else {
        vec![encode_data_frame(ethernet_frame)]
    }
}

/// Heartbeat interval in seconds
const HEARTBEAT_INTERVAL_SECS: u64 = 15;

/// QUIC keep-alive interval in seconds
const QUIC_KEEP_ALIVE_SECS: u64 = 10;

/// Maximum reconnection delay in seconds
const MAX_RECONNECT_DELAY_SECS: u64 = 30;

/// Initial reconnection delay in seconds
const INITIAL_RECONNECT_DELAY_SECS: u64 = 2;

/// Control message for registration
fn make_register_message(mac: &[u8; 6]) -> Vec<u8> {
    let json = format!(
        r#"{{"type":"Register","mac":[{},{},{},{},{},{}]}}"#,
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    let mut msg = Vec::with_capacity(1 + json.len());
    msg.push(MSG_TYPE_CONTROL);
    msg.extend(json.bytes());
    msg
}

/// Control message for heartbeat
fn make_heartbeat_message() -> Vec<u8> {
    let json = r#"{"type":"Heartbeat"}"#;
    let mut msg = Vec::with_capacity(1 + json.len());
    msg.push(MSG_TYPE_CONTROL);
    msg.extend(json.bytes());
    msg
}

/// Encode an Ethernet frame with the data prefix
fn encode_data_frame(ethernet_frame: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(1 + ethernet_frame.len());
    frame.push(MSG_TYPE_DATA);
    frame.extend(ethernet_frame);
    frame
}

/// Decode a received message, stripping the type prefix for data frames
fn decode_message(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return None;
    }

    match data[0] {
        MSG_TYPE_DATA => {
            // Return the Ethernet frame without the prefix
            Some(data[1..].to_vec())
        }
        MSG_TYPE_CONTROL => {
            // Control messages are handled internally
            // if let Ok(json_str) = std::str::from_utf8(&data[1..]) {
            //     if json_str.contains("\"type\":\"Assigned\"") {
            //         log::info!("[WebTransport] Received IP assignment: {}", json_str);
            //     } else if json_str.contains("\"type\":\"HeartbeatAck\"") {
            //         log::trace!("[WebTransport] Heartbeat acknowledged");
            //     } else if json_str.contains("\"type\":\"Error\"") {
            //         log::error!("[WebTransport] Error from relay: {}", json_str);
            //     }
            // }
            None
        }
        _ => {
            log::warn!("[WebTransport] Unknown message type: {}", data[0]);
            None
        }
    }
}

/// Parse IP address from JSON string containing "ip":[a,b,c,d]
fn parse_ip_from_json(json_str: &str) -> Option<[u8; 4]> {
    let start_marker = "\"ip\":[";
    if let Some(start) = json_str.find(start_marker) {
        let rest = &json_str[start + start_marker.len()..];
        if let Some(end) = rest.find(']') {
            let ip_str = &rest[..end];
            let parts: Vec<&str> = ip_str.split(',').collect();
            if parts.len() == 4 {
                let b0 = parts[0].trim().parse().ok()?;
                let b1 = parts[1].trim().parse().ok()?;
                let b2 = parts[2].trim().parse().ok()?;
                let b3 = parts[3].trim().parse().ok()?;
                return Some([b0, b1, b2, b3]);
            }
        }
    }
    None
}

/// Connection status for JavaScript
#[napi]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// WebTransport client for Node.js.
///
/// This class provides WebTransport connectivity using the same implementation
/// as the native Rust VM. It maintains a persistent connection with automatic
/// reconnection and heartbeat handling.
#[napi]
pub struct WebTransportClient {
    tx_to_transport: Option<Sender<Vec<u8>>>,
    rx_from_transport: Option<Receiver<Vec<u8>>>,
    mac: [u8; 6],
    registered: Arc<AtomicBool>,
    assigned_ip: Arc<Mutex<Option<[u8; 4]>>>,
    connection_attempts: Arc<AtomicU32>,
    connected: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    chunk_id_counter: Arc<Mutex<u16>>,
}

#[napi]
impl WebTransportClient {
    /// Create a new WebTransport client and start connecting.
    ///
    /// @param url - WebTransport server URL (e.g., "https://localhost:4433")
    /// @param certHash - Optional certificate hash for self-signed certs (hex string)
    #[napi(constructor)]
    pub fn new(url: String, cert_hash: Option<String>) -> Self {
        // Initialize env_logger if not already done
        let _ = env_logger::try_init();

        log::info!("[WebTransport] Creating client for URL: {}", url);

        // Generate a random MAC address
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let nanos = now.as_nanos() as u64;
        let pid = std::process::id() as u64;
        let url_hash: u64 = url
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let seed = nanos ^ (pid << 32) ^ url_hash;

        let mut mac = [0x52, 0x54, 0x00, 0x00, 0x00, 0x00];
        mac[2] = ((seed >> 40) & 0xff) as u8;
        mac[3] = ((seed >> 32) & 0xff) as u8;
        mac[4] = ((seed >> 16) & 0xff) as u8;
        mac[5] = (seed & 0xff) as u8;

        let (tx_to_transport, rx_to_transport) = channel::<Vec<u8>>();
        let (tx_from_transport, rx_from_transport) = channel::<Vec<u8>>();

        let mac_copy = mac;
        let registered = Arc::new(AtomicBool::new(false));
        let registered_clone = registered.clone();
        let assigned_ip = Arc::new(Mutex::new(None));
        let assigned_ip_clone = assigned_ip.clone();
        let connection_attempts = Arc::new(AtomicU32::new(0));
        let connection_attempts_clone = connection_attempts.clone();
        let connected = Arc::new(AtomicBool::new(false));
        let connected_clone = connected.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                // Parse cert hash once
                let cert_digest = if let Some(hash_str) = &cert_hash {
                    log::info!("[WebTransport] Using certificate hash: {}", hash_str);
                    let bytes = match hex::decode(hash_str.replace(":", "")) {
                        Ok(b) => b,
                        Err(e) => {
                            log::error!("[WebTransport] Invalid hex hash: {}", e);
                            return;
                        }
                    };
                    let bytes_len = bytes.len();
                    let array: [u8; 32] = match bytes.try_into() {
                        Ok(a) => a,
                        Err(_) => {
                            log::error!(
                                "[WebTransport] Hash must be 32 bytes, got {} bytes",
                                bytes_len
                            );
                            return;
                        }
                    };
                    Some(Sha256Digest::from(array))
                } else {
                    log::warn!("[WebTransport] No certificate hash provided, disabling cert validation");
                    None
                };

                let mut reconnect_delay = INITIAL_RECONNECT_DELAY_SECS;

                // Reconnection loop
                loop {
                    // Check for shutdown
                    if shutdown_clone.load(Ordering::SeqCst) {
                        log::info!("[WebTransport] Shutdown requested, exiting");
                        break;
                    }

                    let attempt = connection_attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;

                    if attempt > 1 {
                        log::info!(
                            "[WebTransport] Reconnection attempt {} (delay was {}s)...",
                            attempt,
                            reconnect_delay
                        );
                    } else {
                        log::info!("[WebTransport] Starting connection to {}...", url);
                    }

                    // Reset state
                    registered_clone.store(false, Ordering::SeqCst);
                    connected_clone.store(false, Ordering::SeqCst);

                    // Build config
                    let config = if let Some(ref digest) = cert_digest {
                        ClientConfig::builder()
                            .with_bind_default()
                            .with_server_certificate_hashes(vec![digest.clone()])
                            .keep_alive_interval(Some(Duration::from_secs(QUIC_KEEP_ALIVE_SECS)))
                            .build()
                    } else {
                        ClientConfig::builder()
                            .with_bind_default()
                            .with_no_cert_validation()
                            .keep_alive_interval(Some(Duration::from_secs(QUIC_KEEP_ALIVE_SECS)))
                            .build()
                    };

                    let endpoint = match Endpoint::client(config) {
                        Ok(ep) => ep,
                        Err(e) => {
                            log::error!("[WebTransport] Failed to provision endpoint: {}", e);
                            tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                            reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_SECS);
                            continue;
                        }
                    };

                    log::info!("[WebTransport] Connecting to {}...", url);
                    let connection = match endpoint.connect(&url).await {
                        Ok(conn) => {
                            reconnect_delay = INITIAL_RECONNECT_DELAY_SECS;
                            conn
                        }
                        Err(e) => {
                            log::error!("[WebTransport] Connection failed: {}", e);
                            tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                            reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_SECS);
                            continue;
                        }
                    };

                    connected_clone.store(true, Ordering::SeqCst);

                    // Send registration
                    let register_msg = make_register_message(&mac_copy);
                    if let Err(e) = connection.send_datagram(register_msg) {
                        log::error!("[WebTransport] Failed to send registration: {}", e);
                        connected_clone.store(false, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                        reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_SECS);
                        continue;
                    }


                    let connection = Arc::new(connection);

                    // Combined send/receive/heartbeat loop
                    let mut heartbeat_interval =
                        tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
                    let mut send_check_interval = tokio::time::interval(Duration::from_millis(1));
                    let mut chunk_buffer: std::collections::HashMap<u16, (Vec<Option<Vec<u8>>>, u8, u8)> = std::collections::HashMap::new();

                    'connection_loop: loop {
                        // Check for shutdown
                        if shutdown_clone.load(Ordering::SeqCst) {
                            log::info!("[WebTransport] Shutdown requested during connection");
                            return;
                        }

                        tokio::select! {
                            // Check for data to send
                            _ = send_check_interval.tick() => {
                                loop {
                                    match rx_to_transport.try_recv() {
                                        Ok(data) => {
                                            if let Err(e) = connection.send_datagram(data) {
                                                log::error!("Failed to send datagram: {}", e);
                                                break 'connection_loop;
                                            }
                                        }
                                        Err(TryRecvError::Empty) => break,
                                        Err(TryRecvError::Disconnected) => {
                                            log::warn!("[WebTransport] TX channel disconnected");
                                            return;
                                        }
                                    }
                                }
                            }

                            // Send heartbeats
                            _ = heartbeat_interval.tick() => {
                                let heartbeat = make_heartbeat_message();
                                if let Err(e) = connection.send_datagram(heartbeat) {
                                    log::warn!("[WebTransport] Failed to send heartbeat: {}", e);
                                    break 'connection_loop;
                                }
                                log::trace!("[WebTransport] Heartbeat sent");
                            }

                            result = connection.receive_datagram() => {
                                match result {
                                    Ok(datagram) => {
                                        let data = datagram.to_vec();

                                        // Handle Assigned message
                                        if !data.is_empty() && data[0] == MSG_TYPE_CONTROL {
                                            if let Ok(json_str) = std::str::from_utf8(&data[1..]) {
                                                if json_str.contains("\"type\":\"Assigned\"") {
                                                    registered_clone.store(true, Ordering::SeqCst);

                                                    if let Some(ip) = parse_ip_from_json(json_str) {
                                                        if let Ok(mut guard) = assigned_ip_clone.lock() {
                                                            *guard = Some(ip);
                                                        }
                                                    }

                                                }
                                            }
                                        }

                                        // Forward Ethernet frames
                                        if !data.is_empty() && data[0] == MSG_TYPE_CHUNKED {
                                            if let Some(chunk_info) = decode_chunk(&data) {
                                                let entry = chunk_buffer.entry(chunk_info.chunk_id).or_insert_with(|| {
                                                    (vec![None; chunk_info.total_chunks as usize], chunk_info.total_chunks, 0)
                                                });
                                                
                                                let idx = chunk_info.chunk_index as usize;
                                                if idx < entry.0.len() && entry.0[idx].is_none() {
                                                    entry.0[idx] = Some(chunk_info.payload);
                                                    entry.2 += 1;
                                                    
                                                    if entry.2 == entry.1 {
                                                        let mut complete_frame = Vec::new();
                                                        for chunk in &entry.0 {
                                                            if let Some(data) = chunk {
                                                                complete_frame.extend(data);
                                                            }
                                                        }
                                                        chunk_buffer.remove(&chunk_info.chunk_id);
                                                        log::info!("[WebTransport] Reassembled {} byte frame from {} chunks", complete_frame.len(), chunk_info.total_chunks);
                                                        let _ = tx_from_transport.send(complete_frame);
                                                    }
                                                }
                                            }
                                        } else if let Some(ethernet_frame) = decode_message(&data) {
                                            let _ = tx_from_transport.send(ethernet_frame);
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("[WebTransport] Connection lost: {}", e);
                                        break 'connection_loop;
                                    }
                                }
                            }
                        }
                    }

                    // Connection lost
                    connected_clone.store(false, Ordering::SeqCst);
                    log::info!(
                        "[WebTransport] Scheduling reconnection in {}s...",
                        reconnect_delay
                    );
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_SECS);
                }
            });
        });

        Self {
            tx_to_transport: Some(tx_to_transport),
            rx_from_transport: Some(rx_from_transport),
            mac,
            registered,
            assigned_ip,
            connection_attempts,
            connected,
            shutdown,
            chunk_id_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Check if connected to the relay server.
    #[napi]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Check if registered with the relay (IP assigned).
    #[napi]
    pub fn is_registered(&self) -> bool {
        self.registered.load(Ordering::SeqCst)
    }

    /// Get current connection status.
    #[napi]
    pub fn status(&self) -> ConnectionStatus {
        if self.registered.load(Ordering::SeqCst) {
            ConnectionStatus::Connected
        } else if self.connected.load(Ordering::SeqCst) {
            ConnectionStatus::Connecting
        } else {
            ConnectionStatus::Disconnected
        }
    }

    /// Get the MAC address as a hex string.
    #[napi]
    pub fn mac_address(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
        )
    }

    /// Get the MAC address as a byte array.
    #[napi]
    pub fn mac_bytes(&self) -> Vec<u8> {
        self.mac.to_vec()
    }

    /// Get the assigned IP address (if any) as a string.
    #[napi]
    pub fn assigned_ip(&self) -> Option<String> {
        if let Ok(guard) = self.assigned_ip.lock() {
            guard.map(|ip| format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]))
        } else {
            None
        }
    }

    /// Get the assigned IP address as bytes (if any).
    #[napi]
    pub fn assigned_ip_bytes(&self) -> Option<Vec<u8>> {
        if let Ok(guard) = self.assigned_ip.lock() {
            guard.map(|ip| ip.to_vec())
        } else {
            None
        }
    }

    /// Get the number of connection attempts.
    #[napi]
    pub fn connection_attempts(&self) -> u32 {
        self.connection_attempts.load(Ordering::SeqCst)
    }

    /// Send an Ethernet frame to the relay.
    /// Returns true if sent successfully, false if not connected.
    #[napi]
    pub fn send(&self, data: Buffer) -> bool {
        if let Some(tx) = &self.tx_to_transport {
            // Use smart encoding to chunk if necessary
            let mut id_guard = match self.chunk_id_counter.lock() {
                Ok(g) => g,
                Err(e) => {
                    log::error!("[WebTransport] Failed to lock chunk counter: {:?}", e);
                    return false;
                }
            };
            
            let datagrams = encode_frame_smart(&data, &mut *id_guard);
            
            if datagrams.len() > 1 {
                 log::info!("[WebTransport] Sending {} bytes in {} chunks", data.len(), datagrams.len());
            }

            let mut all_sent = true;
            for datagram in datagrams {
                if tx.send(datagram).is_err() {
                    all_sent = false;
                }
            }
            all_sent
        } else {
            false
        }
    }

    /// Receive an Ethernet frame from the relay (non-blocking).
    /// Returns the frame data or undefined if no frame available.
    #[napi]
    pub fn recv(&self) -> Option<Buffer> {
        if let Some(rx) = &self.rx_from_transport {
            match rx.try_recv() {
                Ok(data) => Some(Buffer::from(data)),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// Receive all pending Ethernet frames (non-blocking).
    /// Returns an array of frame buffers.
    #[napi]
    pub fn recv_all(&self) -> Vec<Buffer> {
        let mut frames = Vec::new();
        if let Some(rx) = &self.rx_from_transport {
            loop {
                match rx.try_recv() {
                    Ok(data) => frames.push(Buffer::from(data)),
                    Err(_) => break,
                }
            }
        }
        frames
    }

    /// Shut down the connection and cleanup.
    #[napi]
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        log::info!("[WebTransport] Shutdown signaled");
    }
}
