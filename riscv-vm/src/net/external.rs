//! External network backend for Node.js native addon bridging.
//!
//! This backend allows JavaScript code to inject/extract packets directly,
//! enabling the use of native Node.js WebTransport addon with the WASM VM.

use super::NetworkBackend;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// External network backend that can be driven from JavaScript.
///
/// Packets are injected via `inject_rx_packet()` and extracted via `extract_tx_packet()`.
/// This enables bridging the WASM VM to native networking code in Node.js.
pub struct ExternalNetworkBackend {
    state: Mutex<ExternalNetState>,
}

struct ExternalNetState {
    /// Queue of packets to be received by the guest (injected from external source)
    rx_queue: VecDeque<Vec<u8>>,
    /// Queue of packets sent by the guest (to be extracted by external sink)
    tx_queue: VecDeque<Vec<u8>>,
    /// MAC address
    mac: [u8; 6],
    /// Assigned IP address (if any)
    assigned_ip: Option<[u8; 4]>,
    /// Whether the backend is connected
    connected: bool,
}

impl ExternalNetworkBackend {
    /// Create a new external network backend with the given MAC address.
    pub fn new(mac: [u8; 6]) -> Self {
        Self {
            state: Mutex::new(ExternalNetState {
                rx_queue: VecDeque::new(),
                tx_queue: VecDeque::new(),
                mac,
                assigned_ip: None,
                connected: false,
            }),
        }
    }

    /// Inject a packet to be received by the guest.
    /// Called from JavaScript when the native addon receives a packet.
    pub fn inject_rx_packet(&self, packet: Vec<u8>) {
        if let Ok(mut state) = self.state.lock() {
            state.rx_queue.push_back(packet);
        }
    }

    /// Extract a packet sent by the guest.
    /// Called from JavaScript to get packets to send via native addon.
    pub fn extract_tx_packet(&self) -> Option<Vec<u8>> {
        if let Ok(mut state) = self.state.lock() {
            state.tx_queue.pop_front()
        } else {
            None
        }
    }

    /// Extract all pending TX packets.
    pub fn extract_all_tx_packets(&self) -> Vec<Vec<u8>> {
        if let Ok(mut state) = self.state.lock() {
            state.tx_queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    /// Set the assigned IP address.
    pub fn set_assigned_ip(&self, ip: [u8; 4]) {
        if let Ok(mut state) = self.state.lock() {
            state.assigned_ip = Some(ip);
        }
    }

    /// Set connection status.
    pub fn set_connected(&self, connected: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.connected = connected;
        }
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        if let Ok(state) = self.state.lock() {
            state.connected
        } else {
            false
        }
    }

    /// Get number of pending RX packets.
    pub fn rx_queue_len(&self) -> usize {
        if let Ok(state) = self.state.lock() {
            state.rx_queue.len()
        } else {
            0
        }
    }

    /// Get number of pending TX packets.
    pub fn tx_queue_len(&self) -> usize {
        if let Ok(state) = self.state.lock() {
            state.tx_queue.len()
        } else {
            0
        }
    }
}

// Make it Send for the NetworkBackend trait
unsafe impl Send for ExternalNetworkBackend {}

impl NetworkBackend for ExternalNetworkBackend {
    fn init(&mut self) -> Result<(), String> {
        if let Ok(mut state) = self.state.lock() {
            state.connected = true;
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
        if let Ok(mut state) = self.state.lock() {
            Ok(state.rx_queue.pop_front())
        } else {
            Err("Failed to lock state".to_string())
        }
    }

    fn send(&self, buf: &[u8]) -> Result<(), String> {
        if let Ok(mut state) = self.state.lock() {
            state.tx_queue.push_back(buf.to_vec());
            Ok(())
        } else {
            Err("Failed to lock state".to_string())
        }
    }

    fn mac_address(&self) -> [u8; 6] {
        if let Ok(state) = self.state.lock() {
            state.mac
        } else {
            [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
        }
    }

    fn get_assigned_ip(&self) -> Option<[u8; 4]> {
        if let Ok(state) = self.state.lock() {
            state.assigned_ip
        } else {
            None
        }
    }
}

/// Wrapper around Arc<ExternalNetworkBackend> that implements NetworkBackend.
/// This is needed because VirtioNet takes Box<dyn NetworkBackend>.
pub struct ExternalBackendWrapper {
    pub inner: Arc<ExternalNetworkBackend>,
}

unsafe impl Send for ExternalBackendWrapper {}

impl NetworkBackend for ExternalBackendWrapper {
    fn init(&mut self) -> Result<(), String> {
        if let Ok(mut state) = self.inner.state.lock() {
            state.connected = true;
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
        if let Ok(mut state) = self.inner.state.lock() {
            Ok(state.rx_queue.pop_front())
        } else {
            Err("Failed to lock state".to_string())
        }
    }

    fn send(&self, buf: &[u8]) -> Result<(), String> {
        if let Ok(mut state) = self.inner.state.lock() {
            state.tx_queue.push_back(buf.to_vec());
            Ok(())
        } else {
            Err("Failed to lock state".to_string())
        }
    }

    fn mac_address(&self) -> [u8; 6] {
        if let Ok(state) = self.inner.state.lock() {
            state.mac
        } else {
            [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
        }
    }

    fn get_assigned_ip(&self) -> Option<[u8; 4]> {
        if let Ok(state) = self.inner.state.lock() {
            state.assigned_ip
        } else {
            None
        }
    }
}
