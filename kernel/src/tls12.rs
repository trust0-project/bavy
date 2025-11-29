//! TLS 1.2 client implementation for HTTPS connections.
//!
//! This module provides TLS 1.2 support using:
//! - ECDHE_RSA_WITH_AES_128_GCM_SHA256 cipher suite
//! - X25519 or P-256 for key exchange
//! - AES-128-GCM for encryption
//! - SHA-256 for PRF and MAC
//!
//! Certificate verification is disabled (NoVerify mode) for simplicity.

use alloc::vec::Vec;
use alloc::vec;
use aes_gcm::{Aes128Gcm, KeyInit, Nonce, aead::AeadInPlace};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use embedded_io::{Read, Write};
use rand_core::RngCore;

// P-256 (secp256r1) support
use p256::ecdh::EphemeralSecret as P256Secret;
use p256::PublicKey as P256PublicKey;
use p256::EncodedPoint;
use p256::elliptic_curve::sec1::FromEncodedPoint;

use crate::tls::{SimpleRng, TlsError, BlockingTcpSocket};

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// TLS 1.2 version bytes
const TLS_VERSION_1_2: [u8; 2] = [0x03, 0x03];

/// TLS record types
const CONTENT_TYPE_CHANGE_CIPHER_SPEC: u8 = 20;
const CONTENT_TYPE_ALERT: u8 = 21;
const CONTENT_TYPE_HANDSHAKE: u8 = 22;
const CONTENT_TYPE_APPLICATION_DATA: u8 = 23;

/// Handshake message types
const HANDSHAKE_CLIENT_HELLO: u8 = 1;
const HANDSHAKE_SERVER_HELLO: u8 = 2;
const HANDSHAKE_CERTIFICATE: u8 = 11;
const HANDSHAKE_SERVER_KEY_EXCHANGE: u8 = 12;
const HANDSHAKE_SERVER_HELLO_DONE: u8 = 14;
const HANDSHAKE_CLIENT_KEY_EXCHANGE: u8 = 16;
const HANDSHAKE_FINISHED: u8 = 20;

/// Cipher suite: TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
const CIPHER_SUITE_ECDHE_RSA_AES128_GCM_SHA256: [u8; 2] = [0xc0, 0x2f];

/// Extension types
const EXT_SERVER_NAME: u16 = 0;
const EXT_EC_POINT_FORMATS: u16 = 11;
const EXT_SUPPORTED_GROUPS: u16 = 10;
const EXT_SIGNATURE_ALGORITHMS: u16 = 13;

/// Named curves
const NAMED_CURVE_SECP256R1: u16 = 23;
const NAMED_CURVE_X25519: u16 = 29;

/// GCM nonce size
const GCM_NONCE_SIZE: usize = 12;
/// GCM tag size
const GCM_TAG_SIZE: usize = 16;
/// GCM explicit nonce size (the part sent with each record)
const GCM_EXPLICIT_NONCE_SIZE: usize = 8;

// ═══════════════════════════════════════════════════════════════════════════════
// TLS 1.2 CONNECTION STATE
// ═══════════════════════════════════════════════════════════════════════════════

/// TLS 1.2 connection state
pub struct Tls12Connection<'a> {
    socket: BlockingTcpSocket<'a>,
    /// Selected curve
    curve: u16,
    /// Our ECDHE private key (X25519)
    x25519_secret: Option<x25519_dalek::EphemeralSecret>,
    /// Our ECDHE private key (P-256)
    p256_secret: Option<P256Secret>,
    /// Our ECDHE public key (format depends on selected curve)
    client_pubkey_x25519: Vec<u8>,
    /// Our ECDHE public key for P-256 (uncompressed format: 0x04 + X + Y)
    client_pubkey_p256: Vec<u8>,
    /// Server's ECDHE public key
    server_pubkey: Option<Vec<u8>>,
    /// Server random (32 bytes)
    server_random: [u8; 32],
    /// Client random (32 bytes)
    client_random: [u8; 32],
    /// Master secret (48 bytes)
    master_secret: [u8; 48],
    /// Client write key
    client_write_key: [u8; 16],
    /// Server write key
    server_write_key: [u8; 16],
    /// Client write IV (implicit nonce)
    client_write_iv: [u8; 4],
    /// Server write IV (implicit nonce)
    server_write_iv: [u8; 4],
    /// Client sequence number for GCM nonce
    client_seq: u64,
    /// Server sequence number for GCM nonce
    server_seq: u64,
    /// Handshake messages for Finished verification
    handshake_hash: Sha256,
    /// Whether encryption is active
    encrypted: bool,
    /// RNG for generating random values
    rng: SimpleRng,
}

impl<'a> Tls12Connection<'a> {
    /// Create a new TLS 1.2 connection
    pub fn new(socket: BlockingTcpSocket<'a>) -> Self {
        let mut rng = SimpleRng::new();
        
        // Generate client random
        let mut client_random = [0u8; 32];
        rng.fill_bytes(&mut client_random);
        
        Self {
            socket,
            curve: 0,
            x25519_secret: None,
            p256_secret: None,
            client_pubkey_x25519: Vec::new(),
            client_pubkey_p256: Vec::new(),
            server_pubkey: None,
            server_random: [0u8; 32],
            client_random,
            master_secret: [0u8; 48],
            client_write_key: [0u8; 16],
            server_write_key: [0u8; 16],
            client_write_iv: [0u8; 4],
            server_write_iv: [0u8; 4],
            client_seq: 0,
            server_seq: 0,
            handshake_hash: Sha256::new(),
            encrypted: false,
            rng,
        }
    }
    
    /// Perform TLS 1.2 handshake
    pub fn handshake(&mut self, hostname: &str) -> Result<(), TlsError> {
        crate::uart::write_line("TLS 1.2: Starting handshake");
        
        // Generate X25519 keypair
        let x25519_secret = x25519_dalek::EphemeralSecret::random_from_rng(&mut self.rng);
        let x25519_public = x25519_dalek::PublicKey::from(&x25519_secret);
        self.x25519_secret = Some(x25519_secret);
        self.client_pubkey_x25519 = x25519_public.as_bytes().to_vec();
        
        // Generate P-256 keypair
        let p256_secret = P256Secret::random(&mut self.rng);
        let p256_public = p256_secret.public_key();
        // Encode as uncompressed point: 0x04 + X (32 bytes) + Y (32 bytes) = 65 bytes
        let encoded = EncodedPoint::from(&p256_public);
        self.client_pubkey_p256 = encoded.as_bytes().to_vec();
        self.p256_secret = Some(p256_secret);
        
        // Step 1: Send ClientHello
        self.send_client_hello(hostname)?;
        
        // Step 2: Receive ServerHello
        self.recv_server_hello()?;
        
        // Step 3: Receive Certificate
        self.recv_certificate()?;
        
        // Step 4: Receive ServerKeyExchange
        self.recv_server_key_exchange()?;
        
        // Step 5: Receive ServerHelloDone
        self.recv_server_hello_done()?;
        
        // Step 6: Send ClientKeyExchange (uses client_pubkey)
        self.send_client_key_exchange()?;
        
        // Step 7: Compute master secret and keys (consumes secret)
        self.compute_keys()?;
        
        // Step 8: Send ChangeCipherSpec
        self.send_change_cipher_spec()?;
        
        // Enable encryption for client->server
        self.encrypted = true;
        
        // Step 9: Send Finished
        self.send_finished()?;
        
        // Step 10: Receive ChangeCipherSpec
        self.recv_change_cipher_spec()?;
        
        // Step 11: Receive Finished
        self.recv_finished()?;
        
        crate::uart::write_line("TLS 1.2: Handshake complete");
        Ok(())
    }
    
    /// Send ClientHello
    fn send_client_hello(&mut self, hostname: &str) -> Result<(), TlsError> {
        let mut hello = Vec::with_capacity(512);
        
        // Client version (TLS 1.2)
        hello.extend_from_slice(&TLS_VERSION_1_2);
        
        // Client random
        hello.extend_from_slice(&self.client_random);
        
        // Session ID (empty)
        hello.push(0);
        
        // Cipher suites
        hello.push(0); // Length high byte
        hello.push(2); // Length low byte (1 cipher suite = 2 bytes)
        hello.extend_from_slice(&CIPHER_SUITE_ECDHE_RSA_AES128_GCM_SHA256);
        
        // Compression methods (null only)
        hello.push(1); // Length
        hello.push(0); // null compression
        
        // Extensions
        let mut extensions = Vec::new();
        
        // Server Name Indication (SNI)
        if !hostname.is_empty() {
            let mut sni = Vec::new();
            // SNI list length
            let sni_list_len = (hostname.len() + 3) as u16;
            sni.extend_from_slice(&sni_list_len.to_be_bytes());
            // Host name type (0)
            sni.push(0);
            // Host name length
            sni.extend_from_slice(&(hostname.len() as u16).to_be_bytes());
            // Host name
            sni.extend_from_slice(hostname.as_bytes());
            
            extensions.extend_from_slice(&EXT_SERVER_NAME.to_be_bytes());
            extensions.extend_from_slice(&(sni.len() as u16).to_be_bytes());
            extensions.extend_from_slice(&sni);
        }
        
        // Supported Groups (curves) - advertise both P-256 and X25519
        // P-256 first for better server compatibility (many servers only support P-256)
        extensions.extend_from_slice(&EXT_SUPPORTED_GROUPS.to_be_bytes());
        extensions.extend_from_slice(&6u16.to_be_bytes()); // Extension length (2 + 2 + 2)
        extensions.extend_from_slice(&4u16.to_be_bytes()); // List length (2 curves * 2 bytes)
        extensions.extend_from_slice(&NAMED_CURVE_SECP256R1.to_be_bytes()); // P-256 first
        extensions.extend_from_slice(&NAMED_CURVE_X25519.to_be_bytes());    // X25519 second
        
        // EC Point Formats
        extensions.extend_from_slice(&EXT_EC_POINT_FORMATS.to_be_bytes());
        extensions.extend_from_slice(&2u16.to_be_bytes()); // Extension length
        extensions.push(1); // List length
        extensions.push(0); // Uncompressed point format
        
        // Signature Algorithms
        extensions.extend_from_slice(&EXT_SIGNATURE_ALGORITHMS.to_be_bytes());
        extensions.extend_from_slice(&4u16.to_be_bytes()); // Extension length
        extensions.extend_from_slice(&2u16.to_be_bytes()); // List length
        extensions.push(0x04); // SHA256
        extensions.push(0x01); // RSA
        
        // Add extensions length
        hello.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        hello.extend_from_slice(&extensions);
        
        // Build handshake message
        let handshake = self.build_handshake_message(HANDSHAKE_CLIENT_HELLO, &hello);
        
        // Update handshake hash
        self.handshake_hash.update(&handshake);
        
        // Send as TLS record
        self.send_record(CONTENT_TYPE_HANDSHAKE, &handshake)?;
        
        crate::uart::write_line("TLS 1.2: Sent ClientHello");
        Ok(())
    }
    
    /// Receive and parse ServerHello
    fn recv_server_hello(&mut self) -> Result<(), TlsError> {
        let record = self.recv_record()?;
        
        if record.content_type != CONTENT_TYPE_HANDSHAKE {
            crate::uart::write_line("TLS 1.2: Expected handshake, got something else");
            return Err(TlsError::TlsProtocolError);
        }
        
        // Update handshake hash
        self.handshake_hash.update(&record.data);
        
        // Parse handshake header
        if record.data.len() < 4 {
            crate::uart::write_line("TLS 1.2: Record too short for handshake header");
            return Err(TlsError::InvalidData);
        }
        
        let msg_type = record.data[0];
        if msg_type != HANDSHAKE_SERVER_HELLO {
            crate::uart::write_str("TLS 1.2: Expected ServerHello (2), got ");
            let mut buf = [0u8; 10];
            let n = crate::tls::format_u16(msg_type as u16, &mut buf);
            crate::uart::write_line(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
            return Err(TlsError::TlsProtocolError);
        }
        
        let msg_len = u32::from_be_bytes([0, record.data[1], record.data[2], record.data[3]]) as usize;
        if record.data.len() < 4 + msg_len {
            crate::uart::write_line("TLS 1.2: Record too short for message");
            return Err(TlsError::InvalidData);
        }
        
        let msg = &record.data[4..4 + msg_len];
        
        // Parse ServerHello
        if msg.len() < 35 {
            crate::uart::write_line("TLS 1.2: ServerHello too short");
            return Err(TlsError::InvalidData);
        }
        
        let mut pos = 2;
        
        // Server random
        self.server_random.copy_from_slice(&msg[pos..pos + 32]);
        pos += 32;
        
        // Session ID
        let session_id_len = msg[pos] as usize;
        pos += 1 + session_id_len;
        
        // Cipher suite
        if pos + 2 > msg.len() {
            crate::uart::write_line("TLS 1.2: No cipher suite in ServerHello");
            return Err(TlsError::InvalidData);
        }
        let cipher = [msg[pos], msg[pos + 1]];
        
        if cipher != CIPHER_SUITE_ECDHE_RSA_AES128_GCM_SHA256 {
            crate::uart::write_line("TLS 1.2: Server selected unsupported cipher suite");
            return Err(TlsError::TlsProtocolError);
        }
        
        crate::uart::write_line("TLS 1.2: Received ServerHello");
        Ok(())
    }
    
    /// Receive Certificate (we don't validate it)
    fn recv_certificate(&mut self) -> Result<(), TlsError> {
        let record = self.recv_record()?;
        
        if record.content_type != CONTENT_TYPE_HANDSHAKE {
            return Err(TlsError::TlsProtocolError);
        }
        
        self.handshake_hash.update(&record.data);
        
        if record.data.is_empty() || record.data[0] != HANDSHAKE_CERTIFICATE {
            return Err(TlsError::TlsProtocolError);
        }
        
        crate::uart::write_line("TLS 1.2: Received Certificate (not validated)");
        Ok(())
    }
    
    /// Receive ServerKeyExchange (ECDHE parameters)
    fn recv_server_key_exchange(&mut self) -> Result<(), TlsError> {
        let record = self.recv_record()?;
        
        if record.content_type != CONTENT_TYPE_HANDSHAKE {
            crate::uart::write_line("TLS 1.2: Expected handshake for ServerKeyExchange");
            return Err(TlsError::TlsProtocolError);
        }
        
        self.handshake_hash.update(&record.data);
        
        if record.data.is_empty() {
            crate::uart::write_line("TLS 1.2: Empty ServerKeyExchange");
            return Err(TlsError::TlsProtocolError);
        }
        
        let msg_type = record.data[0];
        if msg_type != HANDSHAKE_SERVER_KEY_EXCHANGE {
            crate::uart::write_str("TLS 1.2: Expected ServerKeyExchange (12), got ");
            let mut buf = [0u8; 10];
            let n = crate::tls::format_u16(msg_type as u16, &mut buf);
            crate::uart::write_line(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
            return Err(TlsError::TlsProtocolError);
        }
        
        // Parse ECDHE parameters
        // Format: curve_type (1) + named_curve (2) + pubkey_len (1) + pubkey (n) + signature
        let msg_len = u32::from_be_bytes([0, record.data[1], record.data[2], record.data[3]]) as usize;
        
        if record.data.len() < 4 + msg_len {
            crate::uart::write_line("TLS 1.2: ServerKeyExchange truncated");
            return Err(TlsError::InvalidData);
        }
        
        let msg = &record.data[4..4 + msg_len];
        
        if msg.len() < 5 {
            crate::uart::write_line("TLS 1.2: ServerKeyExchange too short");
            return Err(TlsError::InvalidData);
        }
        
        // curve_type should be 3 (named_curve)
        if msg[0] != 3 {
            crate::uart::write_str("TLS 1.2: Unsupported curve type ");
            let mut buf = [0u8; 10];
            let n = crate::tls::format_u16(msg[0] as u16, &mut buf);
            crate::uart::write_line(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
            return Err(TlsError::TlsProtocolError);
        }
        
        // named_curve
        let curve = u16::from_be_bytes([msg[1], msg[2]]);
        self.curve = curve;
        
        // Accept both P-256 (secp256r1) and X25519
        if curve != NAMED_CURVE_X25519 && curve != NAMED_CURVE_SECP256R1 {
            crate::uart::write_str("TLS 1.2: Unsupported curve: ");
            let mut buf = [0u8; 10];
            let n = crate::tls::format_u16(curve, &mut buf);
            crate::uart::write_line(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
            return Err(TlsError::TlsProtocolError);
        }
        
        // Log which curve server selected
        crate::uart::write_str("TLS 1.2: Server selected curve ");
        if curve == NAMED_CURVE_X25519 {
            crate::uart::write_line("X25519");
        } else {
            crate::uart::write_line("P-256 (secp256r1)");
        }
        
        // Public key
        let pubkey_len = msg[3] as usize;
        crate::uart::write_str("TLS 1.2: Server ECDHE pubkey len=");
        let mut buf = [0u8; 10];
        let n = crate::tls::format_u16(pubkey_len as u16, &mut buf);
        crate::uart::write_line(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
        
        if msg.len() < 4 + pubkey_len {
            crate::uart::write_line("TLS 1.2: ServerKeyExchange pubkey truncated");
            return Err(TlsError::InvalidData);
        }
        
        self.server_pubkey = Some(msg[4..4 + pubkey_len].to_vec());
        
        crate::uart::write_line("TLS 1.2: Received ServerKeyExchange");
        Ok(())
    }
    
    /// Receive ServerHelloDone
    fn recv_server_hello_done(&mut self) -> Result<(), TlsError> {
        let record = self.recv_record()?;
        
        if record.content_type != CONTENT_TYPE_HANDSHAKE {
            return Err(TlsError::TlsProtocolError);
        }
        
        self.handshake_hash.update(&record.data);
        
        if record.data.is_empty() || record.data[0] != HANDSHAKE_SERVER_HELLO_DONE {
            return Err(TlsError::TlsProtocolError);
        }
        
        crate::uart::write_line("TLS 1.2: Received ServerHelloDone");
        Ok(())
    }
    
    /// Compute master secret and derive keys
    fn compute_keys(&mut self) -> Result<(), TlsError> {
        // Perform ECDH to get pre-master secret
        let server_pubkey_bytes = self.server_pubkey.as_ref().ok_or_else(|| {
            crate::uart::write_line("TLS 1.2: No server public key");
            TlsError::InvalidData
        })?;
        
        let pre_master_secret = if self.curve == NAMED_CURVE_X25519 {
            // X25519 ECDH
            if server_pubkey_bytes.len() != 32 {
                crate::uart::write_line("TLS 1.2: Invalid X25519 key length");
                return Err(TlsError::InvalidData);
            }
            
            let mut server_key_bytes = [0u8; 32];
            server_key_bytes.copy_from_slice(server_pubkey_bytes);
            let server_public = x25519_dalek::PublicKey::from(server_key_bytes);
            
            let secret = self.x25519_secret.take().ok_or(TlsError::InvalidData)?;
            let shared = secret.diffie_hellman(&server_public);
            shared.to_bytes().to_vec()
        } else if self.curve == NAMED_CURVE_SECP256R1 {
            // P-256 (secp256r1) ECDH
            // Server public key is in uncompressed format: 0x04 + X (32) + Y (32) = 65 bytes
            if server_pubkey_bytes.len() != 65 {
                crate::uart::write_str("TLS 1.2: Invalid P-256 key length: ");
                let mut buf = [0u8; 10];
                let n = crate::tls::format_u16(server_pubkey_bytes.len() as u16, &mut buf);
                crate::uart::write_line(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
                return Err(TlsError::InvalidData);
            }
            
            // Parse the server's public key from uncompressed format
            let server_point = EncodedPoint::from_bytes(server_pubkey_bytes)
                .map_err(|_| {
                    crate::uart::write_line("TLS 1.2: Invalid P-256 point encoding");
                    TlsError::InvalidData
                })?;
            
            let server_public = P256PublicKey::from_encoded_point(&server_point);
            let server_public = if server_public.is_some().into() {
                server_public.unwrap()
            } else {
                crate::uart::write_line("TLS 1.2: Invalid P-256 public key (not on curve)");
                return Err(TlsError::InvalidData);
            };
            
            // Get our P-256 secret and perform ECDH
            let secret = self.p256_secret.take().ok_or_else(|| {
                crate::uart::write_line("TLS 1.2: No P-256 secret key");
                TlsError::InvalidData
            })?;
            
            let shared = secret.diffie_hellman(&server_public);
            // The shared secret is the raw X coordinate (32 bytes)
            shared.raw_secret_bytes().to_vec()
        } else {
            crate::uart::write_line("TLS 1.2: Unsupported curve in compute_keys");
            return Err(TlsError::TlsProtocolError);
        };
        
        // Compute master secret using PRF
        // master_secret = PRF(pre_master_secret, "master secret", client_random + server_random)
        let mut seed = Vec::with_capacity(64);
        seed.extend_from_slice(&self.client_random);
        seed.extend_from_slice(&self.server_random);
        
        prf_sha256(
            &pre_master_secret,
            b"master secret",
            &seed,
            &mut self.master_secret,
        );
        
        // Compute key block
        // key_block = PRF(master_secret, "key expansion", server_random + client_random)
        seed.clear();
        seed.extend_from_slice(&self.server_random);
        seed.extend_from_slice(&self.client_random);
        
        // For AES_128_GCM_SHA256:
        // client_write_key (16) + server_write_key (16) + client_write_IV (4) + server_write_IV (4)
        let mut key_block = [0u8; 40];
        prf_sha256(
            &self.master_secret,
            b"key expansion",
            &seed,
            &mut key_block,
        );
        
        self.client_write_key.copy_from_slice(&key_block[0..16]);
        self.server_write_key.copy_from_slice(&key_block[16..32]);
        self.client_write_iv.copy_from_slice(&key_block[32..36]);
        self.server_write_iv.copy_from_slice(&key_block[36..40]);
        
        // Log which curve was used
        if self.curve == NAMED_CURVE_X25519 {
            crate::uart::write_line("TLS 1.2: Keys computed (X25519)");
        } else {
            crate::uart::write_line("TLS 1.2: Keys computed (P-256)");
        }
        Ok(())
    }
    
    /// Send ClientKeyExchange
    fn send_client_key_exchange(&mut self) -> Result<(), TlsError> {
        // Use the appropriate public key based on selected curve
        let pubkey_bytes = if self.curve == NAMED_CURVE_X25519 {
            &self.client_pubkey_x25519
        } else if self.curve == NAMED_CURVE_SECP256R1 {
            &self.client_pubkey_p256
        } else {
            crate::uart::write_line("TLS 1.2: No curve selected for ClientKeyExchange");
            return Err(TlsError::InternalError);
        };
        
        if pubkey_bytes.is_empty() {
            crate::uart::write_line("TLS 1.2: Client public key is empty");
            return Err(TlsError::InternalError);
        }
        
        // Build message: pubkey_len (1) + pubkey
        let mut msg = Vec::with_capacity(pubkey_bytes.len() + 1);
        msg.push(pubkey_bytes.len() as u8);
        msg.extend_from_slice(pubkey_bytes);
        
        let handshake = self.build_handshake_message(HANDSHAKE_CLIENT_KEY_EXCHANGE, &msg);
        self.handshake_hash.update(&handshake);
        self.send_record(CONTENT_TYPE_HANDSHAKE, &handshake)?;
        
        crate::uart::write_line("TLS 1.2: Sent ClientKeyExchange");
        Ok(())
    }
    
    /// Send ChangeCipherSpec
    fn send_change_cipher_spec(&mut self) -> Result<(), TlsError> {
        self.send_record(CONTENT_TYPE_CHANGE_CIPHER_SPEC, &[1])?;
        crate::uart::write_line("TLS 1.2: Sent ChangeCipherSpec");
        Ok(())
    }
    
    /// Send Finished
    fn send_finished(&mut self) -> Result<(), TlsError> {
        // Compute verify_data = PRF(master_secret, "client finished", Hash(handshake_messages))
        let handshake_hash = self.handshake_hash.clone().finalize();
        
        let mut verify_data = [0u8; 12];
        prf_sha256(
            &self.master_secret,
            b"client finished",
            &handshake_hash,
            &mut verify_data,
        );
        
        let handshake = self.build_handshake_message(HANDSHAKE_FINISHED, &verify_data);
        
        // Update hash BEFORE encryption (but the encrypted version isn't hashed)
        self.handshake_hash.update(&handshake);
        
        // Encrypt and send
        self.send_encrypted_record(CONTENT_TYPE_HANDSHAKE, &handshake)?;
        
        crate::uart::write_line("TLS 1.2: Sent Finished");
        Ok(())
    }
    
    /// Receive ChangeCipherSpec
    fn recv_change_cipher_spec(&mut self) -> Result<(), TlsError> {
        let record = self.recv_record()?;
        
        if record.content_type != CONTENT_TYPE_CHANGE_CIPHER_SPEC {
            return Err(TlsError::TlsProtocolError);
        }
        
        crate::uart::write_line("TLS 1.2: Received ChangeCipherSpec");
        Ok(())
    }
    
    /// Receive Finished
    fn recv_finished(&mut self) -> Result<(), TlsError> {
        // Receive encrypted record
        let record = self.recv_encrypted_record()?;
        
        if record.content_type != CONTENT_TYPE_HANDSHAKE {
            return Err(TlsError::TlsProtocolError);
        }
        
        if record.data.is_empty() || record.data[0] != HANDSHAKE_FINISHED {
            return Err(TlsError::TlsProtocolError);
        }
        
        crate::uart::write_line("TLS 1.2: Received Finished");
        Ok(())
    }
    
    /// Write data over TLS
    pub fn write(&mut self, data: &[u8]) -> Result<usize, TlsError> {
        self.send_encrypted_record(CONTENT_TYPE_APPLICATION_DATA, data)?;
        Ok(data.len())
    }
    
    /// Read data over TLS
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, TlsError> {
        let record = self.recv_encrypted_record()?;
        
        if record.content_type == CONTENT_TYPE_ALERT {
            return Err(TlsError::ConnectionClosed);
        }
        
        if record.content_type != CONTENT_TYPE_APPLICATION_DATA {
            return Err(TlsError::TlsProtocolError);
        }
        
        let len = record.data.len().min(buf.len());
        buf[..len].copy_from_slice(&record.data[..len]);
        Ok(len)
    }
    
    /// Close the TLS connection
    pub fn close(&mut self) -> Result<(), TlsError> {
        // Send close_notify alert
        let alert = [1, 0]; // warning, close_notify
        let _ = self.send_encrypted_record(CONTENT_TYPE_ALERT, &alert);
        self.socket.close();
        Ok(())
    }
    
    // ═══════════════════════════════════════════════════════════════════════
    // HELPER METHODS
    // ═══════════════════════════════════════════════════════════════════════
    
    /// Build a handshake message with header
    fn build_handshake_message(&self, msg_type: u8, data: &[u8]) -> Vec<u8> {
        let mut msg = Vec::with_capacity(data.len() + 4);
        msg.push(msg_type);
        let len = data.len() as u32;
        msg.push((len >> 16) as u8);
        msg.push((len >> 8) as u8);
        msg.push(len as u8);
        msg.extend_from_slice(data);
        msg
    }
    
    /// Send a TLS record (unencrypted)
    fn send_record(&mut self, content_type: u8, data: &[u8]) -> Result<(), TlsError> {
        let mut record = Vec::with_capacity(data.len() + 5);
        record.push(content_type);
        record.extend_from_slice(&TLS_VERSION_1_2);
        record.extend_from_slice(&(data.len() as u16).to_be_bytes());
        record.extend_from_slice(data);
        
        self.socket.write(&record).map_err(|_| TlsError::Io)?;
        self.socket.flush().map_err(|_| TlsError::Io)?;
        Ok(())
    }
    
    /// Send an encrypted TLS record
    fn send_encrypted_record(&mut self, content_type: u8, plaintext: &[u8]) -> Result<(), TlsError> {
        // Build nonce: implicit IV (4 bytes) + explicit nonce (8 bytes)
        let mut nonce = [0u8; GCM_NONCE_SIZE];
        nonce[..4].copy_from_slice(&self.client_write_iv);
        nonce[4..].copy_from_slice(&self.client_seq.to_be_bytes());
        
        // Build additional data: seq_num (8) + content_type (1) + version (2) + length (2)
        let mut aad = [0u8; 13];
        aad[..8].copy_from_slice(&self.client_seq.to_be_bytes());
        aad[8] = content_type;
        aad[9..11].copy_from_slice(&TLS_VERSION_1_2);
        aad[11..13].copy_from_slice(&(plaintext.len() as u16).to_be_bytes());
        
        // Encrypt
        let cipher = Aes128Gcm::new_from_slice(&self.client_write_key)
            .map_err(|_| TlsError::TlsProtocolError)?;
        
        let mut ciphertext = plaintext.to_vec();
        let nonce_obj = Nonce::from_slice(&nonce);
        
        cipher.encrypt_in_place(nonce_obj, &aad, &mut ciphertext)
            .map_err(|_| TlsError::TlsProtocolError)?;
        
        // Build record: explicit_nonce (8) + ciphertext + tag (already appended by encrypt_in_place)
        let record_len = GCM_EXPLICIT_NONCE_SIZE + ciphertext.len();
        let mut record = Vec::with_capacity(record_len + 5);
        record.push(content_type);
        record.extend_from_slice(&TLS_VERSION_1_2);
        record.extend_from_slice(&(record_len as u16).to_be_bytes());
        record.extend_from_slice(&self.client_seq.to_be_bytes()); // explicit nonce
        record.extend_from_slice(&ciphertext);
        
        self.client_seq += 1;
        
        self.socket.write(&record).map_err(|_| TlsError::Io)?;
        self.socket.flush().map_err(|_| TlsError::Io)?;
        Ok(())
    }
    
    /// Receive a TLS record (unencrypted)
    fn recv_record(&mut self) -> Result<TlsRecord, TlsError> {
        // Read header (5 bytes)
        let mut header = [0u8; 5];
        self.read_exact(&mut header)?;
        
        let content_type = header[0];
        let version = [header[1], header[2]];
        let length = u16::from_be_bytes([header[3], header[4]]) as usize;
        
        crate::uart::write_str("TLS 1.2: Reading record - type=");
        let mut buf = [0u8; 10];
        let n = crate::tls::format_u16(content_type as u16, &mut buf);
        crate::uart::write_str(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
        crate::uart::write_str(", ver=");
        let n = crate::tls::format_u16(version[0] as u16, &mut buf);
        crate::uart::write_str(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
        crate::uart::write_str(".");
        let n = crate::tls::format_u16(version[1] as u16, &mut buf);
        crate::uart::write_str(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
        crate::uart::write_str(", len=");
        let n = crate::tls::format_u16(length as u16, &mut buf);
        crate::uart::write_line(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
        
        if length > 16384 + 2048 {
            crate::uart::write_line("TLS 1.2: Record too large");
            return Err(TlsError::InvalidData);
        }
        
        // Read data
        let mut data = vec![0u8; length];
        self.read_exact(&mut data)?;
        
        // Check for alert
        if content_type == CONTENT_TYPE_ALERT && data.len() >= 2 {
            let level = data[0];
            let desc = data[1];
            crate::uart::write_str("TLS 1.2: Alert received (level=");
            let n = crate::tls::format_u16(level as u16, &mut buf);
            crate::uart::write_str(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
            crate::uart::write_str(", desc=");
            let n = crate::tls::format_u16(desc as u16, &mut buf);
            crate::uart::write_str(core::str::from_utf8(&buf[..n]).unwrap_or("?"));
            crate::uart::write_line(")");
            return Err(TlsError::TlsProtocolError);
        }
        
        Ok(TlsRecord { content_type, data })
    }
    
    /// Receive an encrypted TLS record
    fn recv_encrypted_record(&mut self) -> Result<TlsRecord, TlsError> {
        // Read header
        let mut header = [0u8; 5];
        self.read_exact(&mut header)?;
        
        let content_type = header[0];
        let length = u16::from_be_bytes([header[3], header[4]]) as usize;
        
        if length < GCM_EXPLICIT_NONCE_SIZE + GCM_TAG_SIZE {
            return Err(TlsError::InvalidData);
        }
        
        // Read data
        let mut data = vec![0u8; length];
        self.read_exact(&mut data)?;
        
        // Extract explicit nonce
        let explicit_nonce: [u8; 8] = data[..8].try_into().unwrap();
        let mut ciphertext = data[8..].to_vec();
        
        // Build full nonce
        let mut nonce = [0u8; GCM_NONCE_SIZE];
        nonce[..4].copy_from_slice(&self.server_write_iv);
        nonce[4..].copy_from_slice(&explicit_nonce);
        
        // Build AAD
        let plaintext_len = ciphertext.len() - GCM_TAG_SIZE;
        let mut aad = [0u8; 13];
        aad[..8].copy_from_slice(&self.server_seq.to_be_bytes());
        aad[8] = content_type;
        aad[9..11].copy_from_slice(&TLS_VERSION_1_2);
        aad[11..13].copy_from_slice(&(plaintext_len as u16).to_be_bytes());
        
        // Decrypt
        let cipher = Aes128Gcm::new_from_slice(&self.server_write_key)
            .map_err(|_| TlsError::TlsProtocolError)?;
        
        let nonce_obj = Nonce::from_slice(&nonce);
        cipher.decrypt_in_place(nonce_obj, &aad, &mut ciphertext)
            .map_err(|_| TlsError::TlsProtocolError)?;
        
        self.server_seq += 1;
        
        // Remove tag from result
        ciphertext.truncate(plaintext_len);
        let plaintext = ciphertext;
        
        Ok(TlsRecord { content_type, data: plaintext })
    }
    
    /// Read exactly n bytes
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), TlsError> {
        let mut pos = 0;
        while pos < buf.len() {
            let n = self.socket.read(&mut buf[pos..])?;
            if n == 0 {
                return Err(TlsError::ConnectionClosed);
            }
            pos += n;
        }
        Ok(())
    }
}

/// A received TLS record
struct TlsRecord {
    content_type: u8,
    data: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// TLS 1.2 PRF (Pseudo-Random Function)
// ═══════════════════════════════════════════════════════════════════════════════

/// TLS 1.2 PRF using SHA-256
fn prf_sha256(secret: &[u8], label: &[u8], seed: &[u8], output: &mut [u8]) {
    let mut combined_seed = Vec::with_capacity(label.len() + seed.len());
    combined_seed.extend_from_slice(label);
    combined_seed.extend_from_slice(seed);
    
    p_hash::<Sha256>(secret, &combined_seed, output);
}

/// P_hash function for TLS PRF
fn p_hash<D: Digest + Clone>(secret: &[u8], seed: &[u8], output: &mut [u8]) {
    type HmacSha256 = Hmac<Sha256>;
    
    let mut a = {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(secret).unwrap();
        mac.update(seed);
        mac.finalize().into_bytes().to_vec()
    };
    
    let mut pos = 0;
    while pos < output.len() {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(secret).unwrap();
        mac.update(&a);
        mac.update(seed);
        let result = mac.finalize().into_bytes();
        
        let to_copy = (output.len() - pos).min(result.len());
        output[pos..pos + to_copy].copy_from_slice(&result[..to_copy]);
        pos += to_copy;
        
        // A(i+1) = HMAC(secret, A(i))
        let mut mac = <HmacSha256 as Mac>::new_from_slice(secret).unwrap();
        mac.update(&a);
        a = mac.finalize().into_bytes().to_vec();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════════

/// Perform an HTTPS request using TLS 1.2
pub fn https_request_tls12(
    net: &mut crate::net::NetState,
    ip: smoltcp::wire::Ipv4Address,
    port: u16,
    hostname: &str,
    request_bytes: &[u8],
    timeout_ms: i64,
    get_time: fn() -> i64,
) -> Result<Vec<u8>, TlsError> {
    // Create blocking TCP socket and connect
    let mut socket = BlockingTcpSocket::new(net, timeout_ms, get_time);
    socket.connect(ip, port)?;
    
    // Create TLS 1.2 connection
    let mut tls = Tls12Connection::new(socket);
    
    // Perform handshake
    tls.handshake(hostname)?;
    
    // Send request
    tls.write(request_bytes)?;
    
    // Receive response
    let mut response = Vec::with_capacity(8192);
    let mut buf = [0u8; 1024];
    
    loop {
        match tls.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                response.extend_from_slice(&buf[..n]);
                // Check if response is complete
                if is_http_response_complete(&response) {
                    break;
                }
            }
            Err(TlsError::ConnectionClosed) => break,
            Err(e) => {
                let _ = tls.close();
                return Err(e);
            }
        }
    }
    
    let _ = tls.close();
    Ok(response)
}

/// Check if HTTP response is complete
fn is_http_response_complete(data: &[u8]) -> bool {
    // Find header end
    for i in 0..data.len().saturating_sub(3) {
        if data[i] == b'\r' && data[i + 1] == b'\n' 
           && data[i + 2] == b'\r' && data[i + 3] == b'\n' {
            let body_start = i + 4;
            
            // Check Content-Length
            if let Ok(headers) = core::str::from_utf8(&data[..i]) {
                for line in headers.lines() {
                    if line.to_lowercase().starts_with("content-length:") {
                        if let Some(len_str) = line.split(':').nth(1) {
                            if let Ok(content_len) = len_str.trim().parse::<usize>() {
                                return data.len() >= body_start + content_len;
                            }
                        }
                    }
                }
            }
            
            // No Content-Length, assume complete if we have headers
            return data.len() > body_start;
        }
    }
    false
}

