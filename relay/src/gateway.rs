use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Virtual gateway configuration
pub const GATEWAY_IP: [u8; 4] = [10, 0, 2, 2];
pub const GATEWAY_MAC: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

/// NAT session for tracking UDP connections
#[derive(Clone, Debug)]
struct NatUdpSession {
    /// Original source IP (VM's IP)
    src_ip: [u8; 4],
    /// Original source port
    src_port: u16,
    /// External destination IP
    dst_ip: [u8; 4],
    /// External destination port
    dst_port: u16,
    /// Original source MAC
    src_mac: [u8; 6],
    /// Creation time
    created: Instant,
}

/// NAT session for tracking ICMP ping requests
#[derive(Clone, Debug)]
struct NatIcmpSession {
    /// Original source IP (VM's IP)
    #[allow(dead_code)]
    src_ip: [u8; 4],
    /// Original source MAC
    #[allow(dead_code)]
    src_mac: [u8; 6],
    /// ICMP identifier
    #[allow(dead_code)]
    ident: u16,
    /// ICMP sequence number
    #[allow(dead_code)]
    seq: u16,
    /// External destination IP
    #[allow(dead_code)]
    dst_ip: [u8; 4],
    /// Creation time
    created: Instant,
}

/// NAT Gateway state
pub struct NatGateway {
    /// UDP sessions indexed by (external_dst_ip, external_dst_port, src_port)
    udp_sessions: HashMap<(Ipv4Addr, u16, u16), NatUdpSession>,
    /// ICMP sessions indexed by (dst_ip, ident, seq)
    icmp_sessions: HashMap<(Ipv4Addr, u16, u16), NatIcmpSession>,
    /// UDP socket for external DNS/UDP traffic
    pub udp_socket: Option<Arc<UdpSocket>>,
    /// Channel to send NAT responses back to clients
    response_tx: broadcast::Sender<Vec<u8>>,
}

impl NatGateway {
    pub fn new(response_tx: broadcast::Sender<Vec<u8>>) -> Self {
        Self {
            udp_sessions: HashMap::new(),
            icmp_sessions: HashMap::new(),
            udp_socket: None,
            response_tx,
        }
    }

    /// Initialize the UDP socket for external traffic
    pub async fn init(&mut self) -> anyhow::Result<()> {
        // Bind to 0.0.0.0:0 (ephemeral port) to send/receive external traffic
        // This uses standard user-space networking, no special privileges needed.
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        info!("[NAT] UDP socket bound to {}", socket.local_addr()?);
        self.udp_socket = Some(Arc::new(socket));
        Ok(())
    }

    /// Clean up expired sessions (older than 30 seconds)
    pub fn cleanup_expired(&mut self) {
        let timeout = Duration::from_secs(30);
        let now = Instant::now();
        
        self.udp_sessions.retain(|_, session| {
            now.duration_since(session.created) < timeout
        });
        
        self.icmp_sessions.retain(|_, session| {
            now.duration_since(session.created) < timeout
        });
    }

    /// Check if an IP is external (not in 10.0.0.0/8 private range)
    pub fn is_external_ip(ip: &[u8; 4]) -> bool {
        // Internal: 10.x.x.x, 127.x.x.x
        ip[0] != 10 && ip[0] != 127
    }

    /// Process an outbound UDP packet and perform NAT
    pub async fn process_udp_outbound(&mut self, frame: &[u8]) -> Option<()> {
        if frame.len() < 42 {
            return None;
        }

        // Extract IP addresses
        let src_ip: [u8; 4] = frame[26..30].try_into().ok()?;
        let dst_ip: [u8; 4] = frame[30..34].try_into().ok()?;
        
        // Only NAT external traffic
        if !Self::is_external_ip(&dst_ip) {
            return None;
        }

        // Get IP header length
        let ihl = ((frame[14] & 0x0f) * 4) as usize;
        let udp_start = 14 + ihl;
        
        if frame.len() < udp_start + 8 {
            return None;
        }

        // Extract UDP ports
        let src_port = u16::from_be_bytes([frame[udp_start], frame[udp_start + 1]]);
        let dst_port = u16::from_be_bytes([frame[udp_start + 2], frame[udp_start + 3]]);
        let udp_len = u16::from_be_bytes([frame[udp_start + 4], frame[udp_start + 5]]) as usize;

        // Extract source MAC
        let src_mac: [u8; 6] = frame[6..12].try_into().ok()?;

        // Create NAT session
        let dst_addr = Ipv4Addr::new(dst_ip[0], dst_ip[1], dst_ip[2], dst_ip[3]);
        let session = NatUdpSession {
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            src_mac,
            created: Instant::now(),
        };

        // Store session
        self.udp_sessions.insert((dst_addr, dst_port, src_port), session);

        // Extract UDP payload (skip UDP header)
        let payload_start = udp_start + 8;
        let payload_end = std::cmp::min(udp_start + udp_len, frame.len());
        
        if payload_start >= payload_end {
            return None;
        }

        let payload = &frame[payload_start..payload_end];

        // Send to external destination
        if let Some(ref socket) = self.udp_socket {
            let dest = SocketAddrV4::new(dst_addr, dst_port);
            match socket.send_to(payload, dest).await {
                Ok(n) => {
                    debug!("[NAT] Forwarded {} bytes UDP to {} (VM port {})", n, dest, src_port);
                }
                Err(e) => {
                    warn!("[NAT] Failed to send UDP to {}: {}", dest, e);
                }
            }
        }

        Some(())
    }

    /// Process an outbound ICMP ping and perform NAT
    pub async fn process_icmp_outbound(&mut self, frame: &[u8]) -> Option<()> {
        if frame.len() < 42 {
            return None;
        }

        // Extract IP addresses
        let src_ip: [u8; 4] = frame[26..30].try_into().ok()?;
        let dst_ip: [u8; 4] = frame[30..34].try_into().ok()?;
        
        // Only NAT external traffic
        if !Self::is_external_ip(&dst_ip) {
            return None;
        }

        // Check ICMP type is echo request (8)
        if frame[34] != 8 {
            return None;
        }

        // Extract ICMP ident and seq
        let ident = u16::from_be_bytes([frame[38], frame[39]]);
        let seq = u16::from_be_bytes([frame[40], frame[41]]);
        
        // Extract source MAC
        let src_mac: [u8; 6] = frame[6..12].try_into().ok()?;

        let dst_addr = Ipv4Addr::new(dst_ip[0], dst_ip[1], dst_ip[2], dst_ip[3]);

        // Store ICMP session
        let session = NatIcmpSession {
            src_ip,
            src_mac,
            ident,
            seq,
            dst_ip,
            created: Instant::now(),
        };
        self.icmp_sessions.insert((dst_addr, ident, seq), session);

        info!("[NAT] ICMP echo request to {} (ident={}, seq={})", dst_addr, ident, seq);

        // Execute ping in background
        let response_tx = self.response_tx.clone();
        let src_mac_clone = src_mac;
        let src_ip_clone = src_ip;
        
        tokio::spawn(async move {
            // Try to ping using external process
            // This is safe in Docker as long as 'ping' is installed.
            // It does NOT require NET_ADMIN because we are just invoking a user-space tool.
            let output = tokio::process::Command::new("ping")
                .args(["-c", "1", "-W", "3", &dst_addr.to_string()])
                .output()
                .await;
            
            match output {
                Ok(out) if out.status.success() => {
                    // Generate ICMP echo reply frame
                    let reply = generate_icmp_reply_for_nat(
                        &src_mac_clone, &src_ip_clone, &dst_ip, ident, seq
                    );
                    let _ = response_tx.send(reply);
                    info!("[NAT] ICMP echo reply from {} (ident={}, seq={})", dst_addr, ident, seq);
                }
                Ok(out) => {
                    debug!("[NAT] Ping to {} failed: {:?}", dst_addr, out.status);
                }
                Err(e) => {
                    debug!("[NAT] Failed to execute ping to {}: {}", dst_addr, e);
                }
            }
        });

        Some(())
    }

    /// Handle incoming UDP packet from the external socket
    pub fn handle_incoming_udp(&mut self, buf: &[u8], src_addr: std::net::SocketAddr, n: usize) -> Option<Vec<u8>> {
        // Clean up expired sessions periodically
        self.cleanup_expired();
        
        let src_ip = match src_addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return None,
        };
        let src_port = src_addr.port();
        
        let mut found_session = None;
        for session in self.udp_sessions.values() {
            if session.dst_port == src_port {
                let ip_match = session.dst_ip == src_ip.octets();
                let is_dns = src_port == 53;
                if ip_match || is_dns {
                    found_session = Some(session.clone());
                    break;
                }
            }
        }
        
        if let Some(session) = found_session {
            debug!("[NAT] UDP response from {} -> VM port {}", src_addr, session.src_port);
            Some(self.generate_udp_response(&session, &buf[..n]))
        } else {
            None
        }
    }

    /// Generate an Ethernet+IP+UDP frame for a NAT response
    fn generate_udp_response(&self, session: &NatUdpSession, payload: &[u8]) -> Vec<u8> {
        let udp_len = 8 + payload.len();
        let ip_len = 20 + udp_len;
        let frame_len = 14 + ip_len;
        
        let mut frame = vec![0u8; frame_len];
        
        // Ethernet header
        frame[0..6].copy_from_slice(&session.src_mac);  // dst = VM's MAC
        frame[6..12].copy_from_slice(&GATEWAY_MAC);      // src = gateway MAC
        frame[12..14].copy_from_slice(&[0x08, 0x00]);   // ethertype = IPv4
        
        // IP header
        frame[14] = 0x45;  // version + IHL
        frame[15] = 0;      // TOS
        frame[16..18].copy_from_slice(&(ip_len as u16).to_be_bytes());
        frame[18..20].copy_from_slice(&[0x00, 0x00]);  // identification
        frame[20..22].copy_from_slice(&[0x40, 0x00]);  // flags (DF) + fragment
        frame[22] = 64;     // TTL
        frame[23] = 17;     // protocol = UDP
        frame[24..26].copy_from_slice(&[0x00, 0x00]);  // checksum (fill later)
        frame[26..30].copy_from_slice(&session.dst_ip);  // src IP = external server
        frame[30..34].copy_from_slice(&session.src_ip);  // dst IP = VM's IP
        
        // IP checksum
        let ip_checksum = compute_checksum(&frame[14..34]);
        frame[24] = (ip_checksum >> 8) as u8;
        frame[25] = (ip_checksum & 0xff) as u8;
        
        // UDP header
        let udp_start = 34;
        frame[udp_start..udp_start+2].copy_from_slice(&session.dst_port.to_be_bytes());  // src port = external
        frame[udp_start+2..udp_start+4].copy_from_slice(&session.src_port.to_be_bytes()); // dst port = VM's
        frame[udp_start+4..udp_start+6].copy_from_slice(&(udp_len as u16).to_be_bytes());
        frame[udp_start+6..udp_start+8].copy_from_slice(&[0x00, 0x00]);  // checksum (optional)
        
        // UDP payload
        frame[udp_start+8..].copy_from_slice(payload);
        
        frame
    }
}

/// Generate ICMP echo reply frame for NAT response
fn generate_icmp_reply_for_nat(
    dst_mac: &[u8; 6],
    dst_ip: &[u8; 4],
    src_ip: &[u8; 4],
    ident: u16,
    seq: u16,
) -> Vec<u8> {
    let icmp_data = b"RISCV_PING";  // Match kernel's ping data
    let icmp_len = 8 + icmp_data.len();
    let ip_len = 20 + icmp_len;
    let frame_len = 14 + ip_len;
    
    let mut frame = vec![0u8; frame_len];
    
    // Ethernet header
    frame[0..6].copy_from_slice(dst_mac);           // dst = VM's MAC
    frame[6..12].copy_from_slice(&GATEWAY_MAC);    // src = gateway MAC
    frame[12..14].copy_from_slice(&[0x08, 0x00]); // ethertype = IPv4
    
    // IP header
    frame[14] = 0x45;  // version + IHL
    frame[15] = 0;      // TOS
    frame[16..18].copy_from_slice(&(ip_len as u16).to_be_bytes());
    frame[18..20].copy_from_slice(&ident.to_be_bytes());  // identification
    frame[20..22].copy_from_slice(&[0x00, 0x00]);  // flags + fragment
    frame[22] = 64;     // TTL
    frame[23] = 1;      // protocol = ICMP
    frame[24..26].copy_from_slice(&[0x00, 0x00]);  // checksum (fill later)
    frame[26..30].copy_from_slice(src_ip);         // src IP = external server
    frame[30..34].copy_from_slice(dst_ip);         // dst IP = VM's IP
    
    // IP checksum
    let ip_checksum = compute_checksum(&frame[14..34]);
    frame[24] = (ip_checksum >> 8) as u8;
    frame[25] = (ip_checksum & 0xff) as u8;
    
    // ICMP header
    frame[34] = 0;      // type = echo reply
    frame[35] = 0;      // code
    frame[36..38].copy_from_slice(&[0x00, 0x00]);  // checksum (fill later)
    frame[38..40].copy_from_slice(&ident.to_be_bytes());
    frame[40..42].copy_from_slice(&seq.to_be_bytes());
    frame[42..].copy_from_slice(icmp_data);
    
    // ICMP checksum
    let icmp_checksum = compute_checksum(&frame[34..]);
    frame[36] = (icmp_checksum >> 8) as u8;
    frame[37] = (icmp_checksum & 0xff) as u8;
    
    frame
}

// Packet inspection helpers

pub fn is_arp_request_for_gateway(frame: &[u8]) -> bool {
    if frame.len() < 42 { return false; }
    if frame[12] != 0x08 || frame[13] != 0x06 { return false; }
    if frame[20] != 0x00 || frame[21] != 0x01 { return false; }
    frame[38..42] == GATEWAY_IP
}

pub fn generate_arp_reply(request: &[u8]) -> Vec<u8> {
    let mut reply = vec![0u8; 42];
    reply[0..6].copy_from_slice(&request[6..12]);
    reply[6..12].copy_from_slice(&GATEWAY_MAC);
    reply[12..14].copy_from_slice(&[0x08, 0x06]);
    reply[14..16].copy_from_slice(&[0x00, 0x01]);
    reply[16..18].copy_from_slice(&[0x08, 0x00]);
    reply[18] = 6;
    reply[19] = 4;
    reply[20..22].copy_from_slice(&[0x00, 0x02]);
    reply[22..28].copy_from_slice(&GATEWAY_MAC);
    reply[28..32].copy_from_slice(&GATEWAY_IP);
    reply[32..38].copy_from_slice(&request[22..28]);
    reply[38..42].copy_from_slice(&request[28..32]);
    reply
}

pub fn is_icmp_echo_request_to_gateway(frame: &[u8]) -> bool {
    if frame.len() < 42 { return false; }
    if frame[12] != 0x08 || frame[13] != 0x00 { return false; }
    if frame[23] != 1 { return false; }
    if frame[30..34] != GATEWAY_IP { return false; }
    frame[34] == 8
}

pub fn generate_icmp_reply(request: &[u8]) -> Vec<u8> {
    let mut reply = request.to_vec();
    reply[0..6].copy_from_slice(&request[6..12]);
    reply[6..12].copy_from_slice(&GATEWAY_MAC);
    let orig_src_ip: [u8; 4] = request[26..30].try_into().unwrap();
    let orig_dst_ip: [u8; 4] = request[30..34].try_into().unwrap();
    reply[26..30].copy_from_slice(&orig_dst_ip);
    reply[30..34].copy_from_slice(&orig_src_ip);
    reply[24] = 0;
    reply[25] = 0;
    let ip_checksum = compute_checksum(&reply[14..34]);
    reply[24] = (ip_checksum >> 8) as u8;
    reply[25] = (ip_checksum & 0xff) as u8;
    reply[34] = 0;
    reply[36] = 0;
    reply[37] = 0;
    let icmp_data = &reply[34..];
    let checksum = compute_checksum(icmp_data);
    reply[36] = (checksum >> 8) as u8;
    reply[37] = (checksum & 0xff) as u8;
    reply
}

pub fn is_external_ipv4_packet(frame: &[u8]) -> bool {
    if frame.len() < 34 { return false; }
    if frame[12] != 0x08 || frame[13] != 0x00 { return false; }
    let dst_ip = &frame[30..34];
    dst_ip[0] != 10 && dst_ip[0] != 127
}

pub fn is_udp_packet(frame: &[u8]) -> bool {
    if frame.len() < 34 { return false; }
    if frame[12] != 0x08 || frame[13] != 0x00 { return false; }
    frame[23] == 17
}

pub fn is_icmp_packet(frame: &[u8]) -> bool {
    if frame.len() < 34 { return false; }
    if frame[12] != 0x08 || frame[13] != 0x00 { return false; }
    frame[23] == 1
}

fn compute_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

