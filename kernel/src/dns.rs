//! DNS client implementation for hostname resolution.
//!
//! This module provides DNS query building and response parsing
//! to resolve hostnames to IPv4 addresses.

use alloc::vec::Vec;
use smoltcp::wire::Ipv4Address;

/// DNS query type for A records (IPv4 address)
const DNS_TYPE_A: u16 = 1;
/// DNS class for Internet
const DNS_CLASS_IN: u16 = 1;

/// DNS header flags
const DNS_FLAG_RD: u16 = 0x0100; // Recursion Desired
const DNS_FLAG_QR: u16 = 0x8000; // Query/Response (1 = response)

/// DNS response codes
const DNS_RCODE_MASK: u16 = 0x000F;
#[allow(dead_code)]
const DNS_RCODE_OK: u16 = 0;
const DNS_RCODE_NXDOMAIN: u16 = 3;

/// Transaction ID counter
static mut DNS_TRANSACTION_ID: u16 = 0x1234;

/// Get next transaction ID
fn next_transaction_id() -> u16 {
    unsafe {
        DNS_TRANSACTION_ID = DNS_TRANSACTION_ID.wrapping_add(1);
        DNS_TRANSACTION_ID
    }
}

/// Build a DNS query packet for an A record lookup
/// 
/// Returns (transaction_id, query_packet)
pub fn build_query(hostname: &[u8]) -> (u16, Vec<u8>) {
    let txid = next_transaction_id();
    
    // Estimate packet size: header (12) + name (hostname.len() + 2 for length bytes + 1 for null) + qtype (2) + qclass (2)
    let mut packet = Vec::with_capacity(12 + hostname.len() + 5 + 4);
    
    // DNS Header (12 bytes)
    // Transaction ID
    packet.extend_from_slice(&txid.to_be_bytes());
    // Flags: standard query with recursion desired
    packet.extend_from_slice(&DNS_FLAG_RD.to_be_bytes());
    // Question count: 1
    packet.extend_from_slice(&1u16.to_be_bytes());
    // Answer count: 0
    packet.extend_from_slice(&0u16.to_be_bytes());
    // Authority count: 0
    packet.extend_from_slice(&0u16.to_be_bytes());
    // Additional count: 0
    packet.extend_from_slice(&0u16.to_be_bytes());
    
    // Question section
    // QNAME: domain name encoded as labels
    encode_domain_name(hostname, &mut packet);
    
    // QTYPE: A record (1)
    packet.extend_from_slice(&DNS_TYPE_A.to_be_bytes());
    // QCLASS: IN (1)
    packet.extend_from_slice(&DNS_CLASS_IN.to_be_bytes());
    
    (txid, packet)
}

/// Encode a domain name in DNS format (label length prefix format)
/// e.g., "www.google.com" -> [3]www[6]google[3]com[0]
fn encode_domain_name(hostname: &[u8], packet: &mut Vec<u8>) {
    let mut label_start = 0;
    
    for i in 0..=hostname.len() {
        if i == hostname.len() || hostname[i] == b'.' {
            let label_len = i - label_start;
            if label_len > 0 && label_len <= 63 {
                packet.push(label_len as u8);
                packet.extend_from_slice(&hostname[label_start..i]);
            }
            label_start = i + 1;
        }
    }
    
    // Null terminator
    packet.push(0);
}

/// DNS response parsing result
#[derive(Debug)]
pub enum DnsResult {
    /// Successfully resolved to one or more IPv4 addresses
    Resolved(Vec<Ipv4Address>),
    /// Domain does not exist (NXDOMAIN)
    NotFound,
    /// Server error or malformed response
    Error(&'static str),
    /// Response for wrong transaction ID
    WrongId,
}

/// Parse a DNS response packet
pub fn parse_response(packet: &[u8], expected_txid: u16) -> DnsResult {
    // Minimum DNS header size
    if packet.len() < 12 {
        return DnsResult::Error("Packet too short");
    }
    
    // Check transaction ID
    let txid = u16::from_be_bytes([packet[0], packet[1]]);
    if txid != expected_txid {
        return DnsResult::WrongId;
    }
    
    // Check flags
    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    
    // Verify this is a response
    if flags & DNS_FLAG_QR == 0 {
        return DnsResult::Error("Not a response");
    }
    
    // Check response code
    let rcode = flags & DNS_RCODE_MASK;
    if rcode == DNS_RCODE_NXDOMAIN {
        return DnsResult::NotFound;
    }
    if rcode != 0 {
        return DnsResult::Error("DNS server error");
    }
    
    // Get counts
    let qdcount = u16::from_be_bytes([packet[4], packet[5]]) as usize;
    let ancount = u16::from_be_bytes([packet[6], packet[7]]) as usize;
    
    if ancount == 0 {
        return DnsResult::NotFound;
    }
    
    // Skip the header
    let mut pos = 12;
    
    // Skip question section
    for _ in 0..qdcount {
        // Skip QNAME
        pos = match skip_name(packet, pos) {
            Ok(p) => p,
            Err(e) => return e,
        };
        // Skip QTYPE and QCLASS (4 bytes)
        pos += 4;
        if pos > packet.len() {
            return DnsResult::Error("Truncated question");
        }
    }
    
    // Parse answer section
    let mut addresses = Vec::new();
    
    for _ in 0..ancount {
        if pos >= packet.len() {
            break;
        }
        
        // Skip NAME (may be a pointer)
        pos = match skip_name(packet, pos) {
            Ok(p) => p,
            Err(e) => return e,
        };
        
        // Need at least 10 bytes for TYPE, CLASS, TTL, RDLENGTH
        if pos + 10 > packet.len() {
            return DnsResult::Error("Truncated answer");
        }
        
        let rtype = u16::from_be_bytes([packet[pos], packet[pos + 1]]);
        let rclass = u16::from_be_bytes([packet[pos + 2], packet[pos + 3]]);
        // TTL is at pos+4..pos+8 (we skip it)
        let rdlength = u16::from_be_bytes([packet[pos + 8], packet[pos + 9]]) as usize;
        pos += 10;
        
        if pos + rdlength > packet.len() {
            return DnsResult::Error("Truncated RDATA");
        }
        
        // Check if this is an A record (type 1, class IN)
        if rtype == DNS_TYPE_A && rclass == DNS_CLASS_IN && rdlength == 4 {
            let addr = Ipv4Address::new(
                packet[pos],
                packet[pos + 1],
                packet[pos + 2],
                packet[pos + 3],
            );
            addresses.push(addr);
        }
        
        pos += rdlength;
    }
    
    if addresses.is_empty() {
        DnsResult::NotFound
    } else {
        DnsResult::Resolved(addresses)
    }
}

/// Skip a DNS name (handles compression pointers)
/// Returns the position after the name, or Error
fn skip_name(packet: &[u8], mut pos: usize) -> Result<usize, DnsResult> {
    loop {
        if pos >= packet.len() {
            return Err(DnsResult::Error("Name extends past packet"));
        }
        
        let len = packet[pos];
        
        if len == 0 {
            // End of name (null terminator)
            return Ok(pos + 1);
        }
        
        if len & 0xC0 == 0xC0 {
            // Compression pointer (2 bytes) - just skip it
            return Ok(pos + 2);
        }
        
        // Regular label: skip length byte + label content
        pos += 1 + (len as usize);
        
        // Safety check
        if pos > packet.len() {
            return Err(DnsResult::Error("Label extends past packet"));
        }
    }
}

/// High-level DNS resolution function
/// 
/// This performs a DNS lookup using the provided NetState.
/// Returns the first resolved IPv4 address or None on failure.
pub fn resolve(
    net: &mut crate::net::NetState,
    hostname: &[u8],
    dns_server: Ipv4Address,
    timeout_ms: i64,
    get_time_ms: fn() -> i64,
) -> Option<Ipv4Address> {
    use crate::uart;
    
    // Build query
    let (txid, query) = build_query(hostname);
    
    // Send query
    let start_time = get_time_ms();
    if net.udp_send(dns_server, crate::net::DNS_PORT, &query, start_time).is_err() {
        uart::write_line("Failed to send DNS query");
        return None;
    }
    
    // Wait for response with timeout
    let mut buf = [0u8; 512];
    
    loop {
        let now = get_time_ms();
        if now - start_time > timeout_ms {
            uart::write_line("DNS query timed out");
            return None;
        }
        
        // Poll network
        net.poll(now);
        
        // Try to receive response
        if let Some((_src_ip, _src_port, len)) = net.udp_recv(&mut buf, now) {
            match parse_response(&buf[..len], txid) {
                DnsResult::Resolved(addrs) => {
                    return addrs.into_iter().next();
                }
                DnsResult::NotFound => {
                    uart::write_line("DNS: domain not found");
                    return None;
                }
                DnsResult::Error(e) => {
                    uart::write_str("DNS error: ");
                    uart::write_line(e);
                    return None;
                }
                DnsResult::WrongId => {
                    // Ignore responses with wrong transaction ID
                    continue;
                }
            }
        }
        
        // Small delay to avoid busy-waiting
        for _ in 0..10000 {
            core::hint::spin_loop();
        }
    }
}

