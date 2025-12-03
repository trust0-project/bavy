//! External traffic proxy for the relay hub.
//!
//! Handles:
//! - TCP proxy (HTTP, HTTPS connections)
//! - UDP proxy (DNS queries, etc.)
//! - ICMP proxy (ping requests)

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{Mutex, mpsc};

use crate::protocol::GATEWAY_MAC;

/// Session for tracking NAT'ed UDP connections
#[derive(Debug, Clone)]
struct UdpSession {
    /// Original source MAC
    src_mac: [u8; 6],
    /// Original source IP
    src_ip: [u8; 4],
    /// Original source port
    src_port: u16,
    /// External destination IP
    dst_ip: [u8; 4],
    /// External destination port
    dst_port: u16,
    /// Creation time
    created: Instant,
}

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpState {
    SynSent,
    Established,
    FinWait,
    Closed,
}

/// Session for tracking NAT'ed TCP connections
struct TcpSession {
    /// Original source MAC
    src_mac: [u8; 6],
    /// Original source IP
    src_ip: [u8; 4],
    /// Original source port
    src_port: u16,
    /// External destination IP
    dst_ip: [u8; 4],
    /// External destination port
    dst_port: u16,
    /// Connection state
    state: TcpState,
    /// Last sequence number seen from VM
    vm_seq: u32,
    /// Last ack number seen from VM
    vm_ack: u32,
    /// Last sequence number from server
    server_seq: u32,
    /// Last ack number from server
    server_ack: u32,
    /// Channel to send data to the TCP forwarding task
    tx: mpsc::Sender<Vec<u8>>,
    /// Last activity time
    last_activity: Instant,
}

/// TCP connection key
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct TcpKey {
    src_ip: [u8; 4],
    src_port: u16,
    dst_ip: [u8; 4],
    dst_port: u16,
}

/// External traffic proxy
pub struct ExternalProxy {
    /// UDP socket for external traffic
    udp_socket: Mutex<Option<Arc<UdpSocket>>>,
    /// Active UDP sessions (keyed by local port or dst:port combo)
    udp_sessions: Mutex<HashMap<(Ipv4Addr, u16, u16), UdpSession>>,
    /// Active TCP sessions
    tcp_sessions: Mutex<HashMap<TcpKey, TcpSession>>,
    /// Channel to receive responses from TCP connections
    tcp_response_tx: mpsc::Sender<Vec<u8>>,
    tcp_response_rx: Mutex<mpsc::Receiver<Vec<u8>>>,
    /// Session timeout
    session_timeout: Duration,
}

impl ExternalProxy {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(256);
        Self {
            udp_socket: Mutex::new(None),
            udp_sessions: Mutex::new(HashMap::new()),
            tcp_sessions: Mutex::new(HashMap::new()),
            tcp_response_tx: tx,
            tcp_response_rx: Mutex::new(rx),
            session_timeout: Duration::from_secs(120),
        }
    }

    /// Initialize the proxy (bind UDP socket)
    pub async fn init(&self) -> anyhow::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        tracing::info!(
            "External proxy UDP socket bound to {}",
            socket.local_addr()?
        );
        *self.udp_socket.lock().await = Some(Arc::new(socket));
        Ok(())
    }

    /// Get the UDP socket for receiving
    pub async fn udp_socket(&self) -> Option<Arc<UdpSocket>> {
        self.udp_socket.lock().await.clone()
    }

    /// Poll for TCP responses (non-blocking)
    pub async fn poll_tcp_response(&self) -> Option<Vec<u8>> {
        let mut rx = self.tcp_response_rx.lock().await;
        rx.try_recv().ok()
    }

    /// Handle an external-bound packet from a peer
    pub async fn handle_external_packet(&self, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() < 34 {
            return None;
        }

        let protocol = frame[23];

        match protocol {
            1 => self.handle_icmp(frame).await, // ICMP
            6 => self.handle_tcp(frame).await,  // TCP
            17 => self.handle_udp(frame).await, // UDP
            _ => {
                tracing::trace!("Unsupported protocol: {}", protocol);
                None
            }
        }
    }

    /// Handle outbound ICMP (ping) request
    async fn handle_icmp(&self, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() < 42 {
            return None;
        }

        // Check if this is an echo request (type 8)
        if frame[34] != 8 {
            return None;
        }

        let src_mac: [u8; 6] = frame[6..12].try_into().ok()?;
        let src_ip: [u8; 4] = frame[26..30].try_into().ok()?;
        let dst_ip: [u8; 4] = frame[30..34].try_into().ok()?;
        let ident = u16::from_be_bytes([frame[38], frame[39]]);
        let seq = u16::from_be_bytes([frame[40], frame[41]]);

        let dst_addr = Ipv4Addr::from(dst_ip);
        tracing::debug!(
            "ICMP proxy: ping {} (ident={}, seq={})",
            dst_addr,
            ident,
            seq
        );

        // Execute ping using system command
        // This works in Docker without NET_ADMIN capability
        let output = tokio::process::Command::new("ping")
            .args(["-c", "1", "-W", "3", &dst_addr.to_string()])
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                tracing::debug!("ICMP proxy: ping {} succeeded", dst_addr);
                Some(self.generate_icmp_reply(&src_mac, &src_ip, &dst_ip, ident, seq))
            }
            Ok(_) => {
                tracing::debug!(
                    "ICMP proxy: ping {} failed (timeout or unreachable)",
                    dst_addr
                );
                None
            }
            Err(e) => {
                tracing::warn!("ICMP proxy: failed to execute ping: {}", e);
                None
            }
        }
    }

    /// Generate an ICMP echo reply frame
    fn generate_icmp_reply(
        &self,
        dst_mac: &[u8; 6],
        dst_ip: &[u8; 4],
        src_ip: &[u8; 4],
        ident: u16,
        seq: u16,
    ) -> Vec<u8> {
        let icmp_data = b"RISCV_PING"; // Match kernel's ping data
        let icmp_len = 8 + icmp_data.len();
        let ip_len = 20 + icmp_len;
        let frame_len = 14 + ip_len;

        let mut frame = vec![0u8; frame_len];

        // Ethernet header
        frame[0..6].copy_from_slice(dst_mac);
        frame[6..12].copy_from_slice(&GATEWAY_MAC);
        frame[12..14].copy_from_slice(&[0x08, 0x00]);

        // IP header
        frame[14] = 0x45;
        frame[15] = 0;
        frame[16..18].copy_from_slice(&(ip_len as u16).to_be_bytes());
        frame[18..20].copy_from_slice(&ident.to_be_bytes());
        frame[20..22].copy_from_slice(&[0x00, 0x00]);
        frame[22] = 64; // TTL
        frame[23] = 1; // ICMP
        frame[24..26].copy_from_slice(&[0x00, 0x00]); // checksum placeholder
        frame[26..30].copy_from_slice(src_ip);
        frame[30..34].copy_from_slice(dst_ip);

        // IP checksum
        let ip_checksum = compute_checksum(&frame[14..34]);
        frame[24] = (ip_checksum >> 8) as u8;
        frame[25] = (ip_checksum & 0xff) as u8;

        // ICMP header
        frame[34] = 0; // Echo reply
        frame[35] = 0; // Code
        frame[36..38].copy_from_slice(&[0x00, 0x00]); // checksum placeholder
        frame[38..40].copy_from_slice(&ident.to_be_bytes());
        frame[40..42].copy_from_slice(&seq.to_be_bytes());
        frame[42..].copy_from_slice(icmp_data);

        // ICMP checksum
        let icmp_checksum = compute_checksum(&frame[34..]);
        frame[36] = (icmp_checksum >> 8) as u8;
        frame[37] = (icmp_checksum & 0xff) as u8;

        frame
    }

    /// Handle outbound TCP packet
    async fn handle_tcp(&self, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() < 54 {
            return None;
        }

        let src_mac: [u8; 6] = frame[6..12].try_into().ok()?;
        let src_ip: [u8; 4] = frame[26..30].try_into().ok()?;
        let dst_ip: [u8; 4] = frame[30..34].try_into().ok()?;

        // Get IP header length
        let ihl = ((frame[14] & 0x0f) * 4) as usize;
        let tcp_start = 14 + ihl;

        if frame.len() < tcp_start + 20 {
            return None;
        }

        let src_port = u16::from_be_bytes([frame[tcp_start], frame[tcp_start + 1]]);
        let dst_port = u16::from_be_bytes([frame[tcp_start + 2], frame[tcp_start + 3]]);
        let seq_num = u32::from_be_bytes([
            frame[tcp_start + 4],
            frame[tcp_start + 5],
            frame[tcp_start + 6],
            frame[tcp_start + 7],
        ]);
        let ack_num = u32::from_be_bytes([
            frame[tcp_start + 8],
            frame[tcp_start + 9],
            frame[tcp_start + 10],
            frame[tcp_start + 11],
        ]);
        let flags = frame[tcp_start + 13];

        let syn = (flags & 0x02) != 0;
        let ack = (flags & 0x10) != 0;
        let fin = (flags & 0x01) != 0;
        let rst = (flags & 0x04) != 0;

        let key = TcpKey {
            src_ip,
            src_port,
            dst_ip,
            dst_port,
        };

        let dst_addr = Ipv4Addr::from(dst_ip);

        // Calculate payload size
        let tcp_header_len = ((frame[tcp_start + 12] >> 4) * 4) as usize;
        let payload_start = tcp_start + tcp_header_len;
        let payload_len = if frame.len() > payload_start {
            frame.len() - payload_start
        } else {
            0
        };

        tracing::info!(
            "TCP: {}:{} -> {}:{} flags=[{}{}{}{}] seq={} ack={} payload={}",
            Ipv4Addr::from(src_ip),
            src_port,
            dst_addr,
            dst_port,
            if syn { "S" } else { "" },
            if ack { "A" } else { "" },
            if fin { "F" } else { "" },
            if rst { "R" } else { "" },
            seq_num,
            ack_num,
            payload_len,
        );

        // Handle SYN - new connection or retransmission
        if syn && !ack {
            // Check if we already have a session for this connection
            let sessions = self.tcp_sessions.lock().await;
            if let Some(session) = sessions.get(&key) {
                // SYN retransmission - resend SYN-ACK
                tracing::debug!("TCP proxy: SYN retransmission, resending SYN-ACK");
                let synack = Self::build_tcp_packet(
                    &session.src_mac,
                    &session.src_ip,
                    session.src_port,
                    &session.dst_ip,
                    session.dst_port,
                    session.server_seq,
                    seq_num.wrapping_add(1),
                    0x12, // SYN+ACK
                    &[],
                );
                drop(sessions);
                return Some(synack);
            }
            drop(sessions);

            return self
                .handle_tcp_syn(key, src_mac, src_ip, src_port, dst_ip, dst_port, seq_num)
                .await;
        }

        // Handle RST - connection reset
        if rst {
            let mut sessions = self.tcp_sessions.lock().await;
            if let Some(session) = sessions.remove(&key) {
                tracing::debug!("TCP proxy: connection reset by VM");
                drop(session.tx);
            }
            return None;
        }

        // Handle established connection data
        let mut sessions = self.tcp_sessions.lock().await;
        if let Some(session) = sessions.get_mut(&key) {
            session.last_activity = Instant::now();
            session.vm_seq = seq_num;
            session.vm_ack = ack_num;

            // Handle FIN
            if fin {
                session.state = TcpState::FinWait;
                // Send FIN to server via the channel
                let _ = session.tx.try_send(vec![]);

                // Send FIN-ACK back to VM
                let fin_ack = Self::build_tcp_packet(
                    &session.src_mac,
                    &session.src_ip,
                    session.src_port,
                    &session.dst_ip,
                    session.dst_port,
                    session.server_seq,
                    seq_num.wrapping_add(1),
                    0x11, // FIN+ACK
                    &[],
                );
                drop(sessions);
                return Some(fin_ack);
            }

            // Extract TCP payload (payload_start and payload_len already calculated above)
            if payload_len > 0 {
                let packet_end = seq_num.wrapping_add(payload_len as u32);

                // Check if this packet contains any new data
                // A packet can be:
                // 1. Pure retransmission: packet_end <= server_ack
                // 2. Partial overlap: seq_num < server_ack < packet_end (has some new data)
                // 3. New data: seq_num >= server_ack

                let diff_start = session.server_ack.wrapping_sub(seq_num) as i32;
                let diff_end = packet_end.wrapping_sub(session.server_ack) as i32;

                // Pure retransmission: starts before server_ack AND ends at or before server_ack
                let is_pure_retransmission = diff_start > 0
                    && diff_start < (1 << 30)
                    && (diff_end <= 0 || diff_end >= (1 << 30));

                if is_pure_retransmission {
                    tracing::debug!(
                        "TCP proxy: pure retransmission detected (seq={}, end={}, already_acked={}), sending ACK only",
                        seq_num,
                        packet_end,
                        session.server_ack
                    );

                    // Just re-send ACK, don't forward to server
                    let our_seq = session.server_seq.wrapping_add(1);
                    let ack_packet = Self::build_tcp_packet(
                        &session.src_mac,
                        &session.src_ip,
                        session.src_port,
                        &session.dst_ip,
                        session.dst_port,
                        our_seq,
                        session.server_ack, // Use already-acknowledged value
                        0x10,               // ACK
                        &[],
                    );
                    drop(sessions);
                    return Some(ack_packet);
                }

                // Extract only the new data (skip bytes we've already ACKed)
                let payload = if diff_start > 0 && diff_start < (1 << 30) {
                    // Partial retransmission - extract only new portion
                    let skip_bytes = diff_start as usize;
                    tracing::info!(
                        "TCP proxy: partial retransmission - skipping {} already-acked bytes, forwarding {} new bytes",
                        skip_bytes,
                        payload_len - skip_bytes
                    );
                    frame[payload_start + skip_bytes..].to_vec()
                } else {
                    // All new data
                    frame[payload_start..].to_vec()
                };

                // Calculate the expected ACK based on actual packet end
                let expected_ack = packet_end;

                tracing::info!(
                    "TCP proxy: queueing {} bytes for server task (seq={})",
                    payload_len,
                    seq_num
                );

                // Forward data to the server task
                match session.tx.try_send(payload) {
                    Ok(()) => tracing::info!("TCP proxy: data queued successfully"),
                    Err(e) => tracing::error!("TCP proxy: failed to queue data: {}", e),
                }

                // Send immediate ACK back to VM so it doesn't timeout
                // Need to use server_seq + 1 (since SYN-ACK consumed one seq number)
                let our_seq = session.server_seq.wrapping_add(1);

                tracing::info!(
                    "TCP proxy: sending ACK to VM (our_seq={}, acking={})",
                    our_seq,
                    expected_ack
                );

                let ack_packet = Self::build_tcp_packet(
                    &session.src_mac,
                    &session.src_ip,
                    session.src_port,
                    &session.dst_ip,
                    session.dst_port,
                    our_seq,
                    expected_ack,
                    0x10, // ACK
                    &[],
                );

                // Update our tracking of what we've acked
                session.server_ack = expected_ack;

                drop(sessions);
                return Some(ack_packet);
            }
        } else {
            tracing::debug!("TCP proxy: no session for packet (may be stale)");
        }

        None
    }

    /// Handle TCP SYN - establish new connection
    async fn handle_tcp_syn(
        &self,
        key: TcpKey,
        src_mac: [u8; 6],
        src_ip: [u8; 4],
        src_port: u16,
        dst_ip: [u8; 4],
        dst_port: u16,
        seq_num: u32,
    ) -> Option<Vec<u8>> {
        let dst_addr = Ipv4Addr::from(dst_ip);

        tracing::info!("TCP proxy: new connection to {}:{}", dst_addr, dst_port);

        // Try to connect to the external server
        let server_addr = SocketAddrV4::new(dst_addr, dst_port);

        let stream =
            match tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(server_addr))
                .await
            {
                Ok(Ok(stream)) => stream,
                Ok(Err(e)) => {
                    tracing::warn!("TCP proxy: connection failed: {}", e);
                    return Some(self.generate_tcp_rst(
                        &src_mac, &src_ip, src_port, &dst_ip, dst_port, seq_num,
                    ));
                }
                Err(_) => {
                    tracing::warn!("TCP proxy: connection timeout");
                    return Some(self.generate_tcp_rst(
                        &src_mac, &src_ip, src_port, &dst_ip, dst_port, seq_num,
                    ));
                }
            };

        tracing::info!("TCP proxy: connected to {}:{}", dst_addr, dst_port);

        // Create channel for sending data to the forwarding task
        let (tx, rx) = mpsc::channel(64);

        // Initial sequence numbers (use timestamp-based value)
        let server_seq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u32;

        // Store session
        let session = TcpSession {
            src_mac,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            state: TcpState::SynSent,
            vm_seq: seq_num,
            vm_ack: 0,
            server_seq,
            server_ack: seq_num.wrapping_add(1),
            tx,
            last_activity: Instant::now(),
        };

        self.tcp_sessions.lock().await.insert(key, session);

        // Spawn task to handle this connection
        let response_tx = self.tcp_response_tx.clone();
        tokio::spawn(async move {
            Self::tcp_connection_task(
                stream,
                rx,
                response_tx,
                src_mac,
                src_ip,
                src_port,
                dst_ip,
                dst_port,
                server_seq.wrapping_add(1), // Start data seq after SYN
                seq_num.wrapping_add(1),
            )
            .await;
        });

        // Send SYN-ACK back to VM
        Some(self.generate_tcp_synack(
            &src_mac,
            &src_ip,
            src_port,
            &dst_ip,
            dst_port,
            server_seq,
            seq_num.wrapping_add(1),
        ))
    }

    /// Task that handles a single TCP connection
    async fn tcp_connection_task(
        mut stream: TcpStream,
        mut rx: mpsc::Receiver<Vec<u8>>,
        response_tx: mpsc::Sender<Vec<u8>>,
        src_mac: [u8; 6],
        src_ip: [u8; 4],
        src_port: u16,
        dst_ip: [u8; 4],
        dst_port: u16,
        mut seq: u32,
        mut ack: u32,
    ) {
        let mut buf = vec![0u8; 4096];

        tracing::debug!("TCP proxy task: started with seq={}, ack={}", seq, ack);

        loop {
            tokio::select! {
                // Data from server
                result = stream.read(&mut buf) => {
                    match result {
                        Ok(0) => {
                            tracing::info!("TCP proxy task: server closed connection (sending FIN with seq={}, ack={})", seq, ack);
                            // Send FIN to VM
                            let fin = Self::build_tcp_packet(
                                &src_mac, &src_ip, src_port, &dst_ip, dst_port,
                                seq, ack, 0x11, &[], // FIN+ACK
                            );
                            let _ = response_tx.send(fin).await;
                            break;
                        }
                        Ok(n) => {
                            tracing::info!("TCP proxy task: received {} bytes from server, building packet with seq={}, ack={}", n, seq, ack);

                            // Fragment large data to fit in WebTransport datagrams
                            // Max safe payload size is ~1200 bytes, we use 1000 to be safe
                            const MAX_TCP_PAYLOAD: usize = 1000;
                            let data = &buf[..n];
                            let mut offset = 0;

                            while offset < data.len() {
                                let chunk_end = (offset + MAX_TCP_PAYLOAD).min(data.len());
                                let chunk = &data[offset..chunk_end];
                                let is_last = chunk_end == data.len();

                                // PSH only on last fragment, ACK on all
                                let flags = if is_last { 0x18 } else { 0x10 }; // PSH+ACK or just ACK

                                tracing::info!("TCP proxy task: sending chunk len={} with seq={} to {}:{}",
                                    chunk.len(), seq,
                                    std::net::Ipv4Addr::from(src_ip), src_port);

                                let packet = Self::build_tcp_packet(
                                    &src_mac, &src_ip, src_port, &dst_ip, dst_port,
                                    seq, ack, flags, chunk,
                                );
                                seq = seq.wrapping_add(chunk.len() as u32);

                                if let Err(e) = response_tx.send(packet).await {
                                    tracing::error!("TCP proxy task: failed to send to response channel: {}", e);
                                    break;
                                }

                                offset = chunk_end;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("TCP proxy task: read error: {}", e);
                            break;
                        }
                    }
                }

                // Data from VM
                Some(data) = rx.recv() => {
                    if data.is_empty() {
                        // FIN received, close the stream
                        tracing::info!("TCP proxy task: VM requested close");
                        break;
                    }

                    let old_ack = ack;
                    ack = ack.wrapping_add(data.len() as u32);
                    tracing::info!("TCP proxy task: forwarding {} bytes to server (ack {} -> {})", data.len(), old_ack, ack);

                    if let Err(e) = stream.write_all(&data).await {
                        tracing::warn!("TCP proxy task: write error: {}", e);
                        break;
                    }
                    // Note: ACK is sent immediately by handle_tcp, not here
                }
            }
        }

        // Clean shutdown
        let _ = stream.shutdown().await;
    }

    /// Build a TCP packet to send to the VM
    fn build_tcp_packet(
        dst_mac: &[u8; 6],
        dst_ip: &[u8; 4],
        dst_port: u16,
        src_ip: &[u8; 4],
        src_port: u16,
        seq: u32,
        ack: u32,
        flags: u8,
        payload: &[u8],
    ) -> Vec<u8> {
        let tcp_len = 20 + payload.len();
        let ip_len = 20 + tcp_len;
        let frame_len = 14 + ip_len;

        let mut frame = vec![0u8; frame_len];

        // Ethernet header
        frame[0..6].copy_from_slice(dst_mac);
        frame[6..12].copy_from_slice(&GATEWAY_MAC);
        frame[12..14].copy_from_slice(&[0x08, 0x00]);

        // IP header
        frame[14] = 0x45;
        frame[15] = 0;
        frame[16..18].copy_from_slice(&(ip_len as u16).to_be_bytes());
        frame[18..20].copy_from_slice(&[0x00, 0x00]); // identification
        frame[20..22].copy_from_slice(&[0x40, 0x00]); // DF flag
        frame[22] = 64; // TTL
        frame[23] = 6; // TCP
        frame[24..26].copy_from_slice(&[0x00, 0x00]); // checksum placeholder
        frame[26..30].copy_from_slice(src_ip);
        frame[30..34].copy_from_slice(dst_ip);

        // IP checksum
        let ip_checksum = compute_checksum(&frame[14..34]);
        frame[24] = (ip_checksum >> 8) as u8;
        frame[25] = (ip_checksum & 0xff) as u8;

        // TCP header
        let tcp_start = 34;
        frame[tcp_start..tcp_start + 2].copy_from_slice(&src_port.to_be_bytes());
        frame[tcp_start + 2..tcp_start + 4].copy_from_slice(&dst_port.to_be_bytes());
        frame[tcp_start + 4..tcp_start + 8].copy_from_slice(&seq.to_be_bytes());
        frame[tcp_start + 8..tcp_start + 12].copy_from_slice(&ack.to_be_bytes());
        frame[tcp_start + 12] = 0x50; // Data offset = 5 (20 bytes)
        frame[tcp_start + 13] = flags;
        frame[tcp_start + 14..tcp_start + 16].copy_from_slice(&8192u16.to_be_bytes()); // Window
        frame[tcp_start + 16..tcp_start + 18].copy_from_slice(&[0x00, 0x00]); // Checksum placeholder
        frame[tcp_start + 18..tcp_start + 20].copy_from_slice(&[0x00, 0x00]); // Urgent pointer

        // TCP payload
        if !payload.is_empty() {
            frame[tcp_start + 20..].copy_from_slice(payload);
        }

        // TCP checksum (with pseudo-header)
        let tcp_checksum = compute_tcp_checksum(src_ip, dst_ip, &frame[tcp_start..]);
        frame[tcp_start + 16] = (tcp_checksum >> 8) as u8;
        frame[tcp_start + 17] = (tcp_checksum & 0xff) as u8;

        frame
    }

    /// Generate TCP SYN-ACK packet
    fn generate_tcp_synack(
        &self,
        dst_mac: &[u8; 6],
        dst_ip: &[u8; 4],
        dst_port: u16,
        src_ip: &[u8; 4],
        src_port: u16,
        seq: u32,
        ack: u32,
    ) -> Vec<u8> {
        Self::build_tcp_packet(
            dst_mac,
            dst_ip,
            dst_port,
            src_ip,
            src_port,
            seq,
            ack,
            0x12,
            &[],
        )
    }

    /// Generate TCP RST packet
    fn generate_tcp_rst(
        &self,
        dst_mac: &[u8; 6],
        dst_ip: &[u8; 4],
        dst_port: u16,
        src_ip: &[u8; 4],
        src_port: u16,
        ack: u32,
    ) -> Vec<u8> {
        Self::build_tcp_packet(
            dst_mac,
            dst_ip,
            dst_port,
            src_ip,
            src_port,
            0,
            ack.wrapping_add(1),
            0x14,
            &[],
        )
    }

    /// Handle outbound UDP packet
    async fn handle_udp(&self, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() < 42 {
            return None;
        }

        let src_mac: [u8; 6] = frame[6..12].try_into().ok()?;
        let src_ip: [u8; 4] = frame[26..30].try_into().ok()?;
        let dst_ip: [u8; 4] = frame[30..34].try_into().ok()?;

        // Get IP header length
        let ihl = ((frame[14] & 0x0f) * 4) as usize;
        let udp_start = 14 + ihl;

        if frame.len() < udp_start + 8 {
            return None;
        }

        let src_port = u16::from_be_bytes([frame[udp_start], frame[udp_start + 1]]);
        let dst_port = u16::from_be_bytes([frame[udp_start + 2], frame[udp_start + 3]]);
        let udp_len = u16::from_be_bytes([frame[udp_start + 4], frame[udp_start + 5]]) as usize;

        let payload_start = udp_start + 8;
        let payload_end = std::cmp::min(udp_start + udp_len, frame.len());

        if payload_start >= payload_end {
            return None;
        }

        let payload = &frame[payload_start..payload_end];
        let dst_addr = Ipv4Addr::from(dst_ip);

        tracing::debug!(
            "UDP proxy: {}:{} -> {}:{} ({} bytes)",
            Ipv4Addr::from(src_ip),
            src_port,
            dst_addr,
            dst_port,
            payload.len()
        );

        // Store session for response matching
        let session = UdpSession {
            src_mac,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            created: Instant::now(),
        };

        {
            let mut sessions = self.udp_sessions.lock().await;
            sessions.insert((dst_addr, dst_port, src_port), session);
        }

        // Send to external destination
        let socket = self.udp_socket.lock().await;
        if let Some(ref socket) = *socket {
            let dest = SocketAddrV4::new(dst_addr, dst_port);
            match socket.send_to(payload, dest).await {
                Ok(n) => {
                    tracing::debug!("UDP proxy: sent {} bytes to {}", n, dest);
                }
                Err(e) => {
                    tracing::warn!("UDP proxy: send failed: {}", e);
                }
            }
        }

        // For DNS, we need to wait for a response and return it
        // For now, responses are handled asynchronously via handle_incoming_udp
        None
    }

    /// Handle an incoming UDP packet from the external network
    pub async fn handle_incoming_udp(
        &self,
        data: &[u8],
        src_addr: SocketAddr,
        len: usize,
    ) -> Option<Vec<u8>> {
        self.cleanup_expired_sessions().await;

        let src_ip = match src_addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return None,
        };
        let src_port = src_addr.port();

        // Find matching session
        let session = {
            let sessions = self.udp_sessions.lock().await;
            // For DNS responses, the source is the DNS server
            // Try to find a session that matches
            let mut found = None;
            for ((dst_ip, dst_port, _vm_port), session) in sessions.iter() {
                // Match by destination (external server) port
                if *dst_port == src_port {
                    // Check if IP matches or if it's DNS (port 53)
                    if *dst_ip == src_ip || src_port == 53 {
                        found = Some(session.clone());
                        break;
                    }
                }
            }
            found
        };

        if let Some(session) = session {
            tracing::debug!(
                "UDP proxy: response from {} -> VM port {}",
                src_addr,
                session.src_port
            );
            Some(self.generate_udp_response(&session, &data[..len]))
        } else {
            tracing::trace!("UDP proxy: no matching session for {}", src_addr);
            None
        }
    }

    /// Generate a UDP response frame to send back to the VM
    fn generate_udp_response(&self, session: &UdpSession, payload: &[u8]) -> Vec<u8> {
        let udp_len = 8 + payload.len();
        let ip_len = 20 + udp_len;
        let frame_len = 14 + ip_len;

        let mut frame = vec![0u8; frame_len];

        // Ethernet header
        frame[0..6].copy_from_slice(&session.src_mac);
        frame[6..12].copy_from_slice(&GATEWAY_MAC);
        frame[12..14].copy_from_slice(&[0x08, 0x00]);

        // IP header
        frame[14] = 0x45;
        frame[15] = 0;
        frame[16..18].copy_from_slice(&(ip_len as u16).to_be_bytes());
        frame[18..20].copy_from_slice(&[0x00, 0x00]); // identification
        frame[20..22].copy_from_slice(&[0x40, 0x00]); // DF flag
        frame[22] = 64; // TTL
        frame[23] = 17; // UDP
        frame[24..26].copy_from_slice(&[0x00, 0x00]); // checksum placeholder
        frame[26..30].copy_from_slice(&session.dst_ip); // src = external server
        frame[30..34].copy_from_slice(&session.src_ip); // dst = VM

        // IP checksum
        let ip_checksum = compute_checksum(&frame[14..34]);
        frame[24] = (ip_checksum >> 8) as u8;
        frame[25] = (ip_checksum & 0xff) as u8;

        // UDP header
        let udp_start = 34;
        frame[udp_start..udp_start + 2].copy_from_slice(&session.dst_port.to_be_bytes());
        frame[udp_start + 2..udp_start + 4].copy_from_slice(&session.src_port.to_be_bytes());
        frame[udp_start + 4..udp_start + 6].copy_from_slice(&(udp_len as u16).to_be_bytes());
        frame[udp_start + 6..udp_start + 8].copy_from_slice(&[0x00, 0x00]); // checksum optional

        // UDP payload
        frame[udp_start + 8..].copy_from_slice(payload);

        frame
    }

    /// Clean up expired sessions
    async fn cleanup_expired_sessions(&self) {
        let mut udp_sessions = self.udp_sessions.lock().await;
        udp_sessions.retain(|_, session| session.created.elapsed() < self.session_timeout);
        drop(udp_sessions);

        let mut tcp_sessions = self.tcp_sessions.lock().await;
        tcp_sessions.retain(|_, session| session.last_activity.elapsed() < self.session_timeout);
    }
}

impl Default for ExternalProxy {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute Internet checksum
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

/// Compute TCP checksum including pseudo-header
fn compute_tcp_checksum(src_ip: &[u8; 4], dst_ip: &[u8; 4], tcp_segment: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    // Pseudo-header
    sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
    sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;
    sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
    sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;
    sum += 6u32; // Protocol (TCP)
    sum += tcp_segment.len() as u32;

    // TCP segment
    let mut i = 0;
    while i + 1 < tcp_segment.len() {
        sum += u16::from_be_bytes([tcp_segment[i], tcp_segment[i + 1]]) as u32;
        i += 2;
    }
    if i < tcp_segment.len() {
        sum += (tcp_segment[i] as u32) << 8;
    }

    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_creation() {
        let proxy = ExternalProxy::new();
        assert!(proxy.udp_socket().await.is_none());
    }
}
