//! WebTransport network backend with P2P relay protocol support.
//!
//! This backend tunnels Ethernet frames over WebTransport DATAGRAMs
//! using the relay protocol:
//! - 0x00 prefix: Control messages (JSON-encoded)
//! - 0x01 prefix: Ethernet data frames
//! - 0x02 prefix: Chunked data frames for large packets

use super::NetworkBackend;

/// Message type prefix for control messages
const MSG_TYPE_CONTROL: u8 = 0x00;
/// Message type prefix for Ethernet data frames
const MSG_TYPE_DATA: u8 = 0x01;
/// Message type prefix for chunked data frames
const MSG_TYPE_CHUNKED: u8 = 0x02;

/// Heartbeat interval in seconds (reduced for better keepalive in browsers)
const HEARTBEAT_INTERVAL_SECS: u64 = 15;

/// QUIC keep-alive interval in seconds.
/// Client sends QUIC PING frames at this interval to keep the connection alive.
const QUIC_KEEP_ALIVE_SECS: u64 = 10;

/// Maximum safe chunk payload size (leave room for QUIC overhead)
const MAX_CHUNK_PAYLOAD: usize = 900;

/// Threshold for chunking - frames larger than this will be chunked
const CHUNK_THRESHOLD: usize = 950;

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

/// Encode a large Ethernet frame as multiple chunks
fn encode_chunked_frame(ethernet_frame: &[u8], chunk_id: u16) -> Vec<Vec<u8>> {
    let total_chunks = (ethernet_frame.len() + MAX_CHUNK_PAYLOAD - 1) / MAX_CHUNK_PAYLOAD;
    let total_chunks = total_chunks.min(255) as u8;
    
    let mut chunks = Vec::new();
    
    for (i, chunk_data) in ethernet_frame.chunks(MAX_CHUNK_PAYLOAD).enumerate() {
        let mut frame = Vec::with_capacity(5 + chunk_data.len());
        frame.push(MSG_TYPE_CHUNKED);
        frame.extend(&chunk_id.to_be_bytes());
        frame.push(i as u8);
        frame.push(total_chunks);
        frame.extend(chunk_data);
        chunks.push(frame);
    }
    
    chunks
}

/// Smart frame encoder: uses chunking only if needed
fn encode_frame_smart(ethernet_frame: &[u8], chunk_id: &mut u16) -> Vec<Vec<u8>> {
    if ethernet_frame.len() <= CHUNK_THRESHOLD {
        vec![encode_data_frame(ethernet_frame)]
    } else {
        let id = *chunk_id;
        *chunk_id = chunk_id.wrapping_add(1);
        encode_chunked_frame(ethernet_frame, id)
    }
}

/// Decode a received message, stripping the type prefix for data frames
/// Note: This doesn't handle chunked messages - those need separate reassembly
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
            // Control messages are handled internally, not passed to the VM
            // Log assigned IP if present
            if let Ok(json_str) = std::str::from_utf8(&data[1..]) {
                if json_str.contains("\"type\":\"Assigned\"") {
                    log::info!("[WebTransport] Received IP assignment: {}", json_str);
                } else if json_str.contains("\"type\":\"HeartbeatAck\"") {
                    log::trace!("[WebTransport] Heartbeat acknowledged");
                } else if json_str.contains("\"type\":\"Error\"") {
                    log::error!("[WebTransport] Error from relay: {}", json_str);
                }
            }
            None
        }
        MSG_TYPE_CHUNKED => {
            // Chunked messages need special handling - return None here
            // The caller should use try_decode_chunk instead
            None
        }
        _ => {
            log::warn!("[WebTransport] Unknown message type: {}", data[0]);
            None
        }
    }
}

/// Decoded chunk information
#[derive(Debug, Clone)]
struct ChunkInfo {
    chunk_id: u16,
    chunk_index: u8,
    total_chunks: u8,
    payload: Vec<u8>,
}

/// Decode a chunked frame header
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

/// Parse IP address from JSON string containing "ip":[a,b,c,d]
fn parse_ip_from_json(json_str: &str) -> Option<[u8; 4]> {
    // Look for "ip":[ pattern
    let start_marker = "\"ip\":[";
    if let Some(start) = json_str.find(start_marker) {
        let rest = &json_str[start + start_marker.len()..];
        if let Some(end) = rest.find(']') {
            let ip_str = &rest[..end]; // e.g. "10,0,2,15"
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

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
    use std::thread;
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use wtransport::ClientConfig;
    use wtransport::Endpoint;
    use wtransport::tls::Sha256Digest;

    /// Maximum reconnection delay in seconds
    const MAX_RECONNECT_DELAY_SECS: u64 = 30;
    /// Initial reconnection delay in seconds
    const INITIAL_RECONNECT_DELAY_SECS: u64 = 2;

    pub struct WebTransportBackend {
        tx_to_transport: Option<Sender<Vec<u8>>>,
        rx_from_transport: Option<Receiver<Vec<u8>>>,
        mac: [u8; 6],
        registered: Arc<AtomicBool>,
        /// IP address assigned by the relay server
        assigned_ip: Arc<Mutex<Option<[u8; 4]>>>,
        /// Connection attempt counter (for debugging)
        connection_attempts: Arc<AtomicU32>,
        /// Counter for generating chunk IDs when sending large frames
        chunk_id_counter: Arc<Mutex<u16>>,
    }

    impl WebTransportBackend {
        pub fn new(url: &str, cert_hash: Option<String>) -> Self {
            log::warn!("[WebTransport] Creating backend for URL: {}", url);

            // Generate a random MAC address (locally administered, unicast)
            // Use system time + process id for randomness
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let nanos = now.as_nanos() as u64;
            let pid = std::process::id() as u64;

            // Mix in URL hash for additional entropy
            let url_hash: u64 = url
                .bytes()
                .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

            // Combine all sources of entropy
            let seed = nanos ^ (pid << 32) ^ url_hash;

            let mut mac = [0x52, 0x54, 0x00, 0x00, 0x00, 0x00];
            // Set locally administered bit (0x02) and clear multicast bit (0x01)
            mac[0] = 0x52; // Already has locally administered bit set
            mac[1] = 0x54;
            mac[2] = ((seed >> 40) & 0xff) as u8;
            mac[3] = ((seed >> 32) & 0xff) as u8;
            mac[4] = ((seed >> 16) & 0xff) as u8;
            mac[5] = (seed & 0xff) as u8;

            let (tx_to_transport, rx_to_transport) = channel::<Vec<u8>>();
            let (tx_from_transport, rx_from_transport) = channel::<Vec<u8>>();

            let url = url.to_string();
            let mac_copy = mac;
            let registered = Arc::new(AtomicBool::new(false));
            let registered_clone = registered.clone();
            let assigned_ip = Arc::new(Mutex::new(None));
            let assigned_ip_clone = assigned_ip.clone();
            let connection_attempts = Arc::new(AtomicU32::new(0));
            let connection_attempts_clone = connection_attempts.clone();

            thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async move {
                    // Parse cert hash once outside the reconnection loop
                    let cert_digest = if let Some(hash_str) = &cert_hash {
                        log::warn!("[WebTransport] Using certificate hash: {}", hash_str);
                        let bytes = match hex::decode(hash_str.replace(":", "")) {
                            Ok(b) => b,
                            Err(e) => {
                                log::warn!("[WebTransport] ERROR: Invalid hex hash: {}", e);
                                return;
                            }
                        };
                        let bytes_len = bytes.len();
                        let array: [u8; 32] = match bytes.try_into() {
                            Ok(a) => a,
                            Err(_) => {
                                log::warn!("[WebTransport] ERROR: Hash must be 32 bytes, got {} bytes", bytes_len);
                                return;
                            }
                        };
                        Some(Sha256Digest::from(array))
                    } else {
                        log::warn!("[WebTransport] WARNING: No certificate hash provided, disabling cert validation");
                        None
                    };
                    
                    log::warn!("[WebTransport] QUIC keep-alive interval: {}s", QUIC_KEEP_ALIVE_SECS);

                    // Reconnection loop - keeps trying to connect/reconnect forever
                    let mut reconnect_delay = INITIAL_RECONNECT_DELAY_SECS;
                    
                    loop {
                        let attempt = connection_attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
                        
                        if attempt > 1 {
                            log::warn!("[WebTransport] Reconnection attempt {} (delay was {}s)...", attempt, reconnect_delay);
                        } else {
                            log::warn!("[WebTransport] Starting connection to {}...", url);
                        }
                        
                        // Reset registered state on reconnection
                        registered_clone.store(false, Ordering::SeqCst);
                        
                        // Build config for this connection attempt
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
                                log::warn!("[WebTransport] ERROR: Failed to provision endpoint: {}", e);
                                tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                                reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_SECS);
                                continue;
                            }
                        };

                        log::warn!("[WebTransport] Connecting to {}...", url);
                        let connection = match endpoint.connect(&url).await {
                            Ok(conn) => {
                                // Reset delay on successful connection
                                reconnect_delay = INITIAL_RECONNECT_DELAY_SECS;
                                conn
                            }
                            Err(e) => {
                                log::warn!("[WebTransport] ERROR: Connection failed: {}", e);
                                log::error!("[WebTransport] Connection failed: {}", e);
                                tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                                reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_SECS);
                                continue;
                            }
                        };
                        log::warn!("[WebTransport] Connected successfully!");

                        // Send registration message
                        let register_msg = make_register_message(&mac_copy);
                        if let Err(e) = connection.send_datagram(register_msg) {
                            log::warn!("[WebTransport] ERROR: Failed to send registration: {}", e);
                            tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                            reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_SECS);
                            continue;
                        }
                        log::warn!("[WebTransport] Registration sent, MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            mac_copy[0], mac_copy[1], mac_copy[2], mac_copy[3], mac_copy[4], mac_copy[5]);

                        let connection = Arc::new(connection);
                        
                        // Run sender/receiver/heartbeat in a combined loop using select!
                        // This avoids issues with sharing channels across tasks
                        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
                        let mut send_check_interval = tokio::time::interval(Duration::from_millis(1));
                        
                        // Chunk reassembly buffer: chunk_id -> (chunks_vec, total_chunks, received_count)
                        let mut chunk_buffer: std::collections::HashMap<u16, (Vec<Option<Vec<u8>>>, u8, u8)> = std::collections::HashMap::new();
                        
                        'connection_loop: loop {
                            tokio::select! {
                                // Check for data to send to relay
                                _ = send_check_interval.tick() => {
                                    // Drain all pending sends
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
                                                log::warn!("[WebTransport] TX channel disconnected, shutting down");
                                                return; // Permanent shutdown
                                            }
                                        }
                                    }
                                }
                                
                                // Send periodic heartbeats
                                _ = heartbeat_interval.tick() => {
                                    let heartbeat = make_heartbeat_message();
                                    if let Err(e) = connection.send_datagram(heartbeat) {
                                        log::warn!("[WebTransport] Failed to send heartbeat: {}", e);
                                        break 'connection_loop;
                                    }
                                    log::trace!("[WebTransport] Heartbeat sent");
                                }
                                
                                // Receive data from relay
                                result = connection.receive_datagram() => {
                                    match result {
                                        Ok(datagram) => {
                                            let data = datagram.to_vec();
                                            
                                            // Check for Assigned message to confirm registration and extract IP
                                            if !data.is_empty() && data[0] == MSG_TYPE_CONTROL {
                                                if let Ok(json_str) = std::str::from_utf8(&data[1..]) {
                                                    if json_str.contains("\"type\":\"Assigned\"") {
                                                        registered_clone.store(true, Ordering::SeqCst);
                                                        
                                                        // Parse IP from JSON: {"type":"Assigned","ip":[10,0,2,X],...}
                                                        if let Some(ip) = parse_ip_from_json(json_str) {
                                                            if let Ok(mut guard) = assigned_ip_clone.lock() {
                                                                *guard = Some(ip);
                                                            }
                                                            log::warn!("[WebTransport] IP Assigned: {}.{}.{}.{}", 
                                                                ip[0], ip[1], ip[2], ip[3]);
                                                        }
                                                        
                                                        log::warn!("[WebTransport] Registered with relay: {}", json_str);
                                                    }
                                                }
                                            }
                                            
                                            // Handle chunked frames
                                            if !data.is_empty() && data[0] == MSG_TYPE_CHUNKED {
                                                if let Some(chunk_info) = decode_chunk(&data) {
                                                    let entry = chunk_buffer.entry(chunk_info.chunk_id).or_insert_with(|| {
                                                        (vec![None; chunk_info.total_chunks as usize], chunk_info.total_chunks, 0)
                                                    });

                                                    if chunk_info.chunk_index == 0 {
                                                        log::info!("[WebTransport] Received CHUNKED frame start: id={}, total={}", chunk_info.chunk_id, chunk_info.total_chunks);
                                                    }
                                                    
                                                    let idx = chunk_info.chunk_index as usize;
                                                    if idx < entry.0.len() && entry.0[idx].is_none() {
                                                        entry.0[idx] = Some(chunk_info.payload);
                                                        entry.2 += 1;
                                                        
                                                        // Check if all chunks received
                                                        if entry.2 == entry.1 {
                                                            // Reassemble complete frame
                                                            let mut complete_frame = Vec::new();
                                                            for chunk in &entry.0 {
                                                                if let Some(data) = chunk {
                                                                    complete_frame.extend(data);
                                                                }
                                                            }
                                                            chunk_buffer.remove(&chunk_info.chunk_id);
                                                            log::info!("[WebTransport] Reassembled {} byte frame from {} chunks", 
                                                                complete_frame.len(), chunk_info.total_chunks);
                                                            let _ = tx_from_transport.send(complete_frame);
                                                        }
                                                    }
                                                }
                                            } else if let Some(ethernet_frame) = decode_message(&data) {
                                                // Decode and forward regular Ethernet frames
                                                let _ = tx_from_transport.send(ethernet_frame);
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!("[WebTransport] Connection lost: {}", e);
                                            log::error!("[WebTransport] Receive error: {}", e);
                                            break 'connection_loop;
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Connection lost, wait before reconnecting
                        log::warn!("[WebTransport] Scheduling reconnection in {}s...", reconnect_delay);
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
                chunk_id_counter: Arc::new(Mutex::new(0)),
            }
        }

        /// Check if registered with the relay
        pub fn is_registered(&self) -> bool {
            self.registered.load(Ordering::SeqCst)
        }
    }

    impl NetworkBackend for WebTransportBackend {
        fn init(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
            if let Some(rx) = &self.rx_from_transport {
                match rx.try_recv() {
                    Ok(data) => Ok(Some(data)),
                    Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
                    Err(_) => Err("Disconnected".to_string()),
                }
            } else {
                Ok(None)
            }
        }

        fn send(&self, buf: &[u8]) -> Result<(), String> {
            if let Some(tx) = &self.tx_to_transport {
                // Use smart encoding - chunks large frames automatically
                let mut chunk_id = self.chunk_id_counter.lock().map_err(|e| e.to_string())?;
                let datagrams = encode_frame_smart(buf, &mut chunk_id);
                
                if datagrams.len() > 1 {
                    log::info!("[WebTransport] Sending {} bytes in {} chunks", buf.len(), datagrams.len());
                }
                
                for datagram in datagrams {
                    tx.send(datagram).map_err(|e| e.to_string())?;
                }
                Ok(())
            } else {
                Err("Not connected".to_string())
            }
        }

        fn mac_address(&self) -> [u8; 6] {
            self.mac
        }

        fn get_assigned_ip(&self) -> Option<[u8; 4]> {
            if let Ok(guard) = self.assigned_ip.lock() {
                *guard
            } else {
                None
            }
        }

        fn is_connected(&self) -> bool {
            // For native, we're connected if we have registered with the relay
            self.registered.load(std::sync::atomic::Ordering::SeqCst)
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use js_sys::{Array, Uint8Array};
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        ReadableStreamDefaultReader, WebTransport, WebTransportHash, WebTransportOptions,
        WritableStreamDefaultWriter,
    };

    /// Connection state for tracking and reconnection
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum ConnectionState {
        Disconnected,
        Connecting,
        Connected,
    }

    /// Shared state between the backend and async tasks
    struct SharedState {
        rx_queue: VecDeque<Vec<u8>>,
        registered: bool,
        assigned_ip: Option<[u8; 4]>,
        connection_state: ConnectionState,
        /// Counter incremented on each reconnect to invalidate old tasks
        connection_generation: u32,
        /// Heartbeat interval ID for cleanup
        heartbeat_interval_id: Option<i32>,
        /// Chunk reassembly buffer: chunk_id -> (chunks_vec, total_chunks, received_count)
        chunk_buffer: std::collections::HashMap<u16, (Vec<Option<Vec<u8>>>, u8, u8)>,
    }

    pub struct WebTransportBackend {
        url: String,
        cert_hash: Option<String>,
        mac: [u8; 6],
        transport: Rc<RefCell<Option<WebTransport>>>,
        writer: Rc<RefCell<Option<WritableStreamDefaultWriter>>>,
        state: Rc<RefCell<SharedState>>,
        chunk_id_counter: Rc<RefCell<u16>>,
    }

    // WASM is single threaded
    unsafe impl Send for WebTransportBackend {}

    impl WebTransportBackend {
        pub fn new(url: &str, cert_hash: Option<String>) -> Self {
            // Generate a random MAC address using JS Math.random()
            // This ensures each browser tab/VM instance gets a unique MAC
            let rand1 = (js_sys::Math::random() * 0xFFFFFFFFu32 as f64) as u32;
            let rand2 = (js_sys::Math::random() * 0xFFFFu32 as f64) as u32;

            let mut mac = [0x52, 0x54, 0x00, 0x00, 0x00, 0x00];
            // Set locally administered bit (0x02) and clear multicast bit (0x01)
            mac[0] = 0x52; // Already has locally administered bit set
            mac[1] = 0x54;
            mac[2] = ((rand1 >> 24) & 0xff) as u8;
            mac[3] = ((rand1 >> 16) & 0xff) as u8;
            mac[4] = ((rand1 >> 8) & 0xff) as u8;
            mac[5] = (rand2 & 0xff) as u8;

            let state = Rc::new(RefCell::new(SharedState {
                rx_queue: VecDeque::new(),
                registered: false,
                assigned_ip: None,
                connection_state: ConnectionState::Disconnected,
                connection_generation: 0,
                heartbeat_interval_id: None,
                chunk_buffer: std::collections::HashMap::new(),
            }));

            Self {
                url: url.to_string(),
                cert_hash,
                mac,
                transport: Rc::new(RefCell::new(None)),
                writer: Rc::new(RefCell::new(None)),
                state,
                chunk_id_counter: Rc::new(RefCell::new(0)),
            }
        }

        /// Check if registered with the relay
        pub fn is_registered(&self) -> bool {
            self.state.borrow().registered
        }

        /// Check if connected
        pub fn is_connected(&self) -> bool {
            self.state.borrow().connection_state == ConnectionState::Connected
        }

        /// Start the connection process
        fn start_connection(&self) {
            let url = self.url.clone();
            let cert_hash = self.cert_hash.clone();
            let mac = self.mac;
            let state = self.state.clone();
            let transport_rc = self.transport.clone();
            let writer_rc = self.writer.clone();

            // Increment generation and mark as connecting
            {
                let mut s = state.borrow_mut();
                s.connection_generation += 1;
                s.connection_state = ConnectionState::Connecting;
                s.registered = false;
                // Clear old heartbeat interval
                if let Some(id) = s.heartbeat_interval_id.take() {
                    clear_interval(id);
                }
            }
            let generation = state.borrow().connection_generation;

            console_log(&format!(
                "[WebTransport] Starting connection (gen={}) to {}",
                generation, url
            ));

            wasm_bindgen_futures::spawn_local(async move {
                // Check if this connection attempt is still valid
                if state.borrow().connection_generation != generation {
                    console_log("[WebTransport] Connection attempt superseded, aborting");
                    return;
                }

                let options = WebTransportOptions::new();

                if let Some(hash_hex) = &cert_hash {
                    match hex::decode(hash_hex.replace(":", "")) {
                        Ok(bytes) => {
                            let array = Uint8Array::from(&bytes[..]);
                            let hash_obj = WebTransportHash::new();
                            hash_obj.set_algorithm("sha-256");
                            hash_obj.set_value(&array);
                            let hashes = Array::new();
                            hashes.push(&hash_obj);
                            options.set_server_certificate_hashes(&hashes);
                        }
                        Err(e) => {
                            console_error(&format!("[WebTransport] Invalid cert hash: {}", e));
                            state.borrow_mut().connection_state = ConnectionState::Disconnected;
                            return;
                        }
                    }
                }

                let transport = match WebTransport::new_with_options(&url, &options) {
                    Ok(t) => t,
                    Err(e) => {
                        console_error(&format!(
                            "[WebTransport] Failed to create transport: {:?}",
                            e
                        ));
                        state.borrow_mut().connection_state = ConnectionState::Disconnected;
                        // Schedule reconnection
                        schedule_reconnect(
                            state.clone(),
                            transport_rc.clone(),
                            writer_rc.clone(),
                            url.clone(),
                            cert_hash.clone(),
                            mac,
                            5000,
                        );
                        return;
                    }
                };

                let datagrams = transport.datagrams();
                let writable = datagrams.writable();
                let writer = match writable.get_writer() {
                    Ok(w) => w,
                    Err(e) => {
                        console_error(&format!("[WebTransport] Failed to get writer: {:?}", e));
                        state.borrow_mut().connection_state = ConnectionState::Disconnected;
                        schedule_reconnect(
                            state.clone(),
                            transport_rc.clone(),
                            writer_rc.clone(),
                            url.clone(),
                            cert_hash.clone(),
                            mac,
                            5000,
                        );
                        return;
                    }
                };

                let ready_promise = transport.ready();

                match JsFuture::from(ready_promise).await {
                    Ok(_) => {
                        // Check generation again
                        if state.borrow().connection_generation != generation {
                            console_log("[WebTransport] Connection superseded during handshake");
                            return;
                        }

                        console_log("[WebTransport] Connected successfully!");

                        // Send registration
                        let register_msg = make_register_message(&mac);
                        let array = Uint8Array::from(&register_msg[..]);
                        if let Err(e) = JsFuture::from(writer.write_with_chunk(&array)).await {
                            console_error(&format!("[WebTransport] Failed to register: {:?}", e));
                            state.borrow_mut().connection_state = ConnectionState::Disconnected;
                            schedule_reconnect(
                                state.clone(),
                                transport_rc.clone(),
                                writer_rc.clone(),
                                url.clone(),
                                cert_hash.clone(),
                                mac,
                                5000,
                            );
                            return;
                        }

                        console_log(&format!(
                            "[WebTransport] Registration sent, MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                        ));

                        // Store transport and writer
                        *transport_rc.borrow_mut() = Some(transport.clone());
                        *writer_rc.borrow_mut() = Some(writer.clone());

                        // Setup heartbeat with visibility-aware interval
                        let writer_hb = writer.clone();
                        let state_hb = state.clone();
                        let generation_hb = generation;

                        let heartbeat_closure = Closure::wrap(Box::new(move || {
                            // Only send if still same generation
                            if state_hb.borrow().connection_generation == generation_hb {
                                let heartbeat = make_heartbeat_message();
                                let array = Uint8Array::from(&heartbeat[..]);
                                let _ = writer_hb.write_with_chunk(&array);
                            }
                        })
                            as Box<dyn Fn()>);

                        let interval_id = set_interval(
                            &heartbeat_closure,
                            (HEARTBEAT_INTERVAL_SECS * 1000) as i32,
                        );
                        heartbeat_closure.forget();
                        state.borrow_mut().heartbeat_interval_id = Some(interval_id);

                        // Setup visibility change handler for immediate heartbeat on tab focus
                        setup_visibility_handler(writer.clone(), state.clone(), generation);

                        // Mark as connected
                        state.borrow_mut().connection_state = ConnectionState::Connected;

                        // Start reader loop
                        let readable = transport.datagrams().readable();
                        let reader: ReadableStreamDefaultReader =
                            readable.get_reader().unchecked_into();

                        loop {
                            // Check if we should stop
                            if state.borrow().connection_generation != generation {
                                console_log(
                                    "[WebTransport] Reader loop: generation changed, stopping",
                                );
                                break;
                            }

                            match JsFuture::from(reader.read()).await {
                                Ok(result) => {
                                    let done =
                                        js_sys::Reflect::get(&result, &JsValue::from_str("done"))
                                            .unwrap()
                                            .as_bool()
                                            .unwrap_or(true);
                                    if done {
                                        console_log("[WebTransport] Reader stream ended");
                                        break;
                                    }

                                    let value =
                                        js_sys::Reflect::get(&result, &JsValue::from_str("value"))
                                            .unwrap();
                                    let array = Uint8Array::new(&value);
                                    let data = array.to_vec();

                                    // Handle control messages
                                    if !data.is_empty() && data[0] == MSG_TYPE_CONTROL {
                                        if let Ok(json_str) = std::str::from_utf8(&data[1..]) {
                                            if json_str.contains("\"type\":\"Assigned\"") {
                                                let mut s = state.borrow_mut();
                                                s.registered = true;
                                                if let Some(ip) = parse_ip_from_json(json_str) {
                                                    s.assigned_ip = Some(ip);
                                                    drop(s);
                                                    console_log(&format!(
                                                        "[WebTransport] IP Assigned: {}.{}.{}.{}",
                                                        ip[0], ip[1], ip[2], ip[3]
                                                    ));
                                                }
                                            } else if json_str.contains("\"type\":\"Error\"") {
                                                console_error(&format!(
                                                    "[WebTransport] Relay error: {}",
                                                    json_str
                                                ));
                                            }
                                        }
                                    }

                                    // Handle chunked frames
                                    if !data.is_empty() && data[0] == MSG_TYPE_CHUNKED {
                                        if let Some(chunk_info) = decode_chunk(&data) {
                                            // Debug log for chunk receipt
                                            if chunk_info.chunk_index == 0 {
                                                 console_log(&format!("[WebTransport] Received CHUNKED frame start: id={}, total={}", chunk_info.chunk_id, chunk_info.total_chunks));
                                            }

                                            let mut s = state.borrow_mut();
                                            let entry = s.chunk_buffer.entry(chunk_info.chunk_id).or_insert_with(|| {
                                                (vec![None; chunk_info.total_chunks as usize], chunk_info.total_chunks, 0)
                                            });
                                            
                                            let idx = chunk_info.chunk_index as usize;
                                            if idx < entry.0.len() && entry.0[idx].is_none() {
                                                entry.0[idx] = Some(chunk_info.payload);
                                                entry.2 += 1;
                                                
                                                // Check if all chunks received
                                                if entry.2 == entry.1 {
                                                    // Reassemble complete frame
                                                    let mut complete_frame = Vec::new();
                                                    for chunk in &entry.0 {
                                                        if let Some(data) = chunk {
                                                            complete_frame.extend(data);
                                                        }
                                                    }
                                                    // Remove from buffer
                                                    let id = chunk_info.chunk_id;
                                                    s.chunk_buffer.remove(&id);
                                                    console_log(&format!("[WebTransport] Reassembled frame: {} bytes", complete_frame.len()));
                                                    s.rx_queue.push_back(complete_frame);
                                                }
                                            }
                                        }
                                    } else if let Some(frame) = decode_message(&data) {
                                        state.borrow_mut().rx_queue.push_back(frame);
                                    }
                                }
                                Err(e) => {
                                    console_error(&format!("[WebTransport] Read error: {:?}", e));
                                    break;
                                }
                            }
                        }

                        // Connection ended - cleanup and reconnect
                        console_log("[WebTransport] Connection lost, scheduling reconnection...");
                        {
                            let mut s = state.borrow_mut();
                            if s.connection_generation == generation {
                                s.connection_state = ConnectionState::Disconnected;
                                s.registered = false;
                                if let Some(id) = s.heartbeat_interval_id.take() {
                                    clear_interval(id);
                                }
                            }
                        }
                        *transport_rc.borrow_mut() = None;
                        *writer_rc.borrow_mut() = None;

                        // Only reconnect if this is still the current generation
                        if state.borrow().connection_generation == generation {
                            schedule_reconnect(
                                state,
                                transport_rc,
                                writer_rc,
                                url,
                                cert_hash,
                                mac,
                                3000,
                            );
                        }
                    }
                    Err(e) => {
                        console_error(&format!("[WebTransport] Failed to connect: {:?}", e));
                        state.borrow_mut().connection_state = ConnectionState::Disconnected;
                        schedule_reconnect(
                            state.clone(),
                            transport_rc.clone(),
                            writer_rc.clone(),
                            url.clone(),
                            cert_hash.clone(),
                            mac,
                            5000,
                        );
                    }
                }
            });
        }
    }

    // Helper to log to browser console
    fn console_log(msg: &str) {
        web_sys::console::log_1(&JsValue::from_str(msg));
    }

    fn console_error(msg: &str) {
        web_sys::console::error_1(&JsValue::from_str(msg));
    }

    /// Set up a JS interval and return its ID
    fn set_interval(closure: &Closure<dyn Fn()>, ms: i32) -> i32 {
        let global = js_sys::global();
        let set_interval = js_sys::Reflect::get(&global, &JsValue::from_str("setInterval"))
            .expect("setInterval should exist");
        let set_interval_fn: js_sys::Function = set_interval.unchecked_into();
        let result = set_interval_fn
            .call2(&JsValue::NULL, closure.as_ref(), &JsValue::from(ms))
            .unwrap_or(JsValue::from(0));
        result.as_f64().unwrap_or(0.0) as i32
    }

    /// Clear a JS interval
    fn clear_interval(id: i32) {
        let global = js_sys::global();
        if let Ok(clear) = js_sys::Reflect::get(&global, &JsValue::from_str("clearInterval")) {
            let clear_fn: js_sys::Function = clear.unchecked_into();
            let _ = clear_fn.call1(&JsValue::NULL, &JsValue::from(id));
        }
    }

    /// Set up a JS timeout and return its ID
    fn set_timeout(closure: &Closure<dyn FnMut()>, ms: i32) -> i32 {
        let global = js_sys::global();
        let set_timeout = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
            .expect("setTimeout should exist");
        let set_timeout_fn: js_sys::Function = set_timeout.unchecked_into();
        let result = set_timeout_fn
            .call2(&JsValue::NULL, closure.as_ref(), &JsValue::from(ms))
            .unwrap_or(JsValue::from(0));
        result.as_f64().unwrap_or(0.0) as i32
    }

    /// Schedule a reconnection attempt
    fn schedule_reconnect(
        state: Rc<RefCell<SharedState>>,
        transport_rc: Rc<RefCell<Option<WebTransport>>>,
        writer_rc: Rc<RefCell<Option<WritableStreamDefaultWriter>>>,
        url: String,
        cert_hash: Option<String>,
        mac: [u8; 6],
        delay_ms: i32,
    ) {
        console_log(&format!(
            "[WebTransport] Scheduling reconnect in {}ms...",
            delay_ms
        ));

        let closure = Closure::once(move || {
            // Create a temporary backend to trigger reconnection
            let backend = WebTransportBackend {
                url: url.clone(),
                cert_hash: cert_hash.clone(),
                mac,
                transport: transport_rc,
                writer: writer_rc,
                state,
                chunk_id_counter: Rc::new(RefCell::new(0)),
            };
            backend.start_connection();
        });

        set_timeout(&closure, delay_ms);
        closure.forget();
    }

    /// Setup visibility change handler to send heartbeat when tab becomes visible
    fn setup_visibility_handler(
        writer: WritableStreamDefaultWriter,
        state: Rc<RefCell<SharedState>>,
        generation: u32,
    ) {
        let closure = Closure::wrap(Box::new(move || {
            // Check if document is visible
            let global = js_sys::global();
            if let Ok(document) = js_sys::Reflect::get(&global, &JsValue::from_str("document")) {
                if let Ok(hidden) = js_sys::Reflect::get(&document, &JsValue::from_str("hidden")) {
                    if !hidden.as_bool().unwrap_or(true) {
                        // Tab became visible - send immediate heartbeat
                        if state.borrow().connection_generation == generation {
                            console_log("[WebTransport] Tab visible - sending immediate heartbeat");
                            let heartbeat = make_heartbeat_message();
                            let array = Uint8Array::from(&heartbeat[..]);
                            let _ = writer.write_with_chunk(&array);
                        }
                    }
                }
            }
        }) as Box<dyn Fn()>);

        // Add event listener
        let global = js_sys::global();
        if let Ok(document) = js_sys::Reflect::get(&global, &JsValue::from_str("document")) {
            if let Ok(add_listener) =
                js_sys::Reflect::get(&document, &JsValue::from_str("addEventListener"))
            {
                let add_fn: js_sys::Function = add_listener.unchecked_into();
                let _ = add_fn.call2(
                    &document,
                    &JsValue::from_str("visibilitychange"),
                    closure.as_ref(),
                );
            }
        }
        closure.forget();
    }

    impl NetworkBackend for WebTransportBackend {
        fn init(&mut self) -> Result<(), String> {
            console_log(&format!(
                "[WebTransport] Initializing connection to {}",
                self.url
            ));
            self.start_connection();
            Ok(())
        }

        fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
            Ok(self.state.borrow_mut().rx_queue.pop_front())
        }

        fn send(&self, buf: &[u8]) -> Result<(), String> {
            if let Some(writer) = self.writer.borrow().as_ref() {
                // Use smart encoding - chunks large frames automatically
                let mut id_counter = self.chunk_id_counter.borrow_mut();
                let datagrams = encode_frame_smart(buf, &mut *id_counter);
                
                if datagrams.len() > 1 {
                    console_log(&format!("[WebTransport] Sending {} bytes in {} chunks", buf.len(), datagrams.len()));
                }

                for datagram in datagrams {
                    let array = Uint8Array::from(&datagram[..]);
                    let _ = writer.write_with_chunk(&array);
                }
                Ok(())
            } else {
                Err("Not connected".to_string())
            }
        }

        fn mac_address(&self) -> [u8; 6] {
            self.mac
        }

        fn get_assigned_ip(&self) -> Option<[u8; 4]> {
            self.state.borrow().assigned_ip
        }

        fn is_connected(&self) -> bool {
            self.state.borrow().connection_state == ConnectionState::Connected
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::WebTransportBackend;

#[cfg(target_arch = "wasm32")]
pub use wasm::WebTransportBackend;
