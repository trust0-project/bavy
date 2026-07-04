//! Cryptographic primitives for secure vault encryption.
//!
//! This module provides:
//! - Key derivation using Argon2id (memory-hard, brute-force resistant)
//! - Authenticated encryption using AES-256-GCM
//! - Secure random generation
//!
//! ## Security Properties
//!
//! - **Argon2id**: Hybrid of Argon2i (side-channel resistant) and Argon2d (GPU resistant)
//! - **AES-256-GCM**: Authenticated encryption with associated data (AEAD)
//! - **Zeroize**: Sensitive data is cleared from memory when dropped
//!
//! ## Usage
//!
//! ```rust,ignore
//! use riscv_vm::crypto::{derive_key, encrypt, decrypt, generate_salt, generate_nonce};
//!
//! let passphrase = "my secret passphrase";
//! let salt = generate_salt();
//! let key = derive_key(passphrase.as_bytes(), &salt)?;
//!
//! let plaintext = b"sensitive data";
//! let nonce = generate_nonce();
//! let ciphertext = encrypt(&key, &nonce, plaintext)?;
//!
//! let decrypted = decrypt(&key, &nonce, &ciphertext)?;
//! assert_eq!(plaintext.as_slice(), decrypted.as_slice());
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Vault format version for future compatibility.
pub const VAULT_VERSION: u8 = 1;

/// Salt size for Argon2 (16 bytes = 128 bits).
pub const SALT_SIZE: usize = 16;

/// Nonce size for AES-256-GCM (12 bytes = 96 bits).
pub const NONCE_SIZE: usize = 12;

/// Key size for AES-256 (32 bytes = 256 bits).
pub const KEY_SIZE: usize = 32;

/// Authentication tag size for AES-GCM (16 bytes = 128 bits).
pub const TAG_SIZE: usize = 16;

// ═══════════════════════════════════════════════════════════════════════════════
// ARGON2 PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Argon2 memory cost in KiB.
/// 64 MiB provides strong brute-force resistance while remaining usable on most devices.
/// This can be reduced for constrained environments (e.g., 16 MiB for WASM).
pub const ARGON2_MEMORY_KIB: u32 = 65536; // 64 MiB

/// Argon2 time cost (iterations).
/// Higher values increase computation time linearly.
pub const ARGON2_ITERATIONS: u32 = 3;

/// Argon2 parallelism.
/// Set to 1 for WASM compatibility (single-threaded).
pub const ARGON2_PARALLELISM: u32 = 1;

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Cryptographic operation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Key derivation failed.
    KeyDerivationFailed(String),
    /// Encryption failed.
    EncryptionFailed(String),
    /// Decryption failed (wrong key or corrupted data).
    DecryptionFailed(String),
    /// Invalid input data.
    InvalidInput(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::KeyDerivationFailed(msg) => write!(f, "Key derivation failed: {}", msg),
            CryptoError::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
            CryptoError::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
            CryptoError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for CryptoError {}

// ═══════════════════════════════════════════════════════════════════════════════
// SECURE KEY TYPE
// ═══════════════════════════════════════════════════════════════════════════════

/// A 256-bit encryption key that is automatically zeroed when dropped.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecureKey {
    bytes: [u8; KEY_SIZE],
}

impl SecureKey {
    /// Create a new SecureKey from raw bytes.
    pub fn new(bytes: [u8; KEY_SIZE]) -> Self {
        Self { bytes }
    }

    /// Access the raw key bytes (use with care).
    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &self.bytes
    }
}

impl std::fmt::Debug for SecureKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key bytes, even in debug mode
        write!(f, "SecureKey([REDACTED])")
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// RANDOM GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a cryptographically secure random salt.
pub fn generate_salt() -> [u8; SALT_SIZE] {
    let mut salt = [0u8; SALT_SIZE];
    let mut rng = StdRng::from_entropy();
    rng.fill_bytes(&mut salt);
    salt
}

/// Generate a cryptographically secure random nonce.
pub fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    let mut rng = StdRng::from_entropy();
    rng.fill_bytes(&mut nonce);
    nonce
}

/// Generate a cryptographically secure random key (for testing or ephemeral use).
pub fn generate_random_key() -> SecureKey {
    let mut bytes = [0u8; KEY_SIZE];
    let mut rng = StdRng::from_entropy();
    rng.fill_bytes(&mut bytes);
    SecureKey::new(bytes)
}

// ═══════════════════════════════════════════════════════════════════════════════
// KEY DERIVATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Derive a 256-bit encryption key from a passphrase using Argon2id.
///
/// # Arguments
///
/// * `passphrase` - User-provided passphrase (can be UTF-8 string or raw bytes)
/// * `salt` - Unique salt for this vault (must be stored with ciphertext)
///
/// # Returns
///
/// A `SecureKey` that is automatically zeroed when dropped.
///
/// # Security Notes
///
/// - Uses Argon2id for resistance against both side-channel and GPU attacks
/// - Memory cost is tuned for security vs. usability tradeoff
/// - Salt must be unique per vault and stored alongside ciphertext
pub fn derive_key(passphrase: &[u8], salt: &[u8; SALT_SIZE]) -> Result<SecureKey, CryptoError> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(KEY_SIZE),
    )
    .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key_bytes = [0u8; KEY_SIZE];
    argon2
        .hash_password_into(passphrase, salt, &mut key_bytes)
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Argon2 hash failed: {}", e)))?;

    Ok(SecureKey::new(key_bytes))
}

/// Derive key with custom Argon2 parameters (for testing or constrained environments).
pub fn derive_key_with_params(
    passphrase: &[u8],
    salt: &[u8; SALT_SIZE],
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
) -> Result<SecureKey, CryptoError> {
    let params = Params::new(memory_kib, iterations, parallelism, Some(KEY_SIZE))
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key_bytes = [0u8; KEY_SIZE];
    argon2
        .hash_password_into(passphrase, salt, &mut key_bytes)
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Argon2 hash failed: {}", e)))?;

    Ok(SecureKey::new(key_bytes))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ENCRYPTION / DECRYPTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Encrypt data using AES-256-GCM.
///
/// # Arguments
///
/// * `key` - 256-bit encryption key (from `derive_key`)
/// * `nonce` - 96-bit nonce (must be unique per encryption with same key)
/// * `plaintext` - Data to encrypt
///
/// # Returns
///
/// Ciphertext with appended 16-byte authentication tag.
///
/// # Security Notes
///
/// - Never reuse a nonce with the same key
/// - The authentication tag ensures integrity and authenticity
/// - Ciphertext length = plaintext length + 16 bytes (tag)
pub fn encrypt(
    key: &SecureKey,
    nonce: &[u8; NONCE_SIZE],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| CryptoError::EncryptionFailed(format!("Cipher init failed: {}", e)))?;

    let gcm_nonce = Nonce::from_slice(nonce);

    cipher
        .encrypt(gcm_nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(format!("Encryption failed: {}", e)))
}

/// Decrypt data using AES-256-GCM.
///
/// # Arguments
///
/// * `key` - 256-bit encryption key (from `derive_key`)
/// * `nonce` - 96-bit nonce (must match the nonce used for encryption)
/// * `ciphertext` - Encrypted data with authentication tag
///
/// # Returns
///
/// Decrypted plaintext.
///
/// # Errors
///
/// Returns `DecryptionFailed` if:
/// - The key is wrong
/// - The ciphertext has been tampered with
/// - The nonce doesn't match
pub fn decrypt(
    key: &SecureKey,
    nonce: &[u8; NONCE_SIZE],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < TAG_SIZE {
        return Err(CryptoError::DecryptionFailed(
            "Ciphertext too short (missing auth tag)".to_string(),
        ));
    }

    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| CryptoError::DecryptionFailed(format!("Cipher init failed: {}", e)))?;

    let gcm_nonce = Nonce::from_slice(nonce);

    cipher
        .decrypt(gcm_nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed("Authentication failed".to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ENCRYPTED VAULT FORMAT
// ═══════════════════════════════════════════════════════════════════════════════

/// Header for encrypted vault format.
#[derive(Debug, Clone)]
pub struct VaultHeader {
    /// Format version for future compatibility.
    pub version: u8,
    /// Unique salt for key derivation.
    pub salt: [u8; SALT_SIZE],
    /// Unique nonce for encryption.
    pub nonce: [u8; NONCE_SIZE],
}

impl VaultHeader {
    /// Total header size in bytes.
    pub const SIZE: usize = 1 + SALT_SIZE + NONCE_SIZE; // 1 + 16 + 12 = 29 bytes

    /// Serialize header to bytes.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.version;
        bytes[1..1 + SALT_SIZE].copy_from_slice(&self.salt);
        bytes[1 + SALT_SIZE..].copy_from_slice(&self.nonce);
        bytes
    }

    /// Deserialize header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() < Self::SIZE {
            return Err(CryptoError::InvalidInput(format!(
                "Header too short: {} < {}",
                bytes.len(),
                Self::SIZE
            )));
        }

        let version = bytes[0];
        if version != VAULT_VERSION {
            return Err(CryptoError::InvalidInput(format!(
                "Unsupported vault version: {} (expected {})",
                version, VAULT_VERSION
            )));
        }

        let mut salt = [0u8; SALT_SIZE];
        salt.copy_from_slice(&bytes[1..1 + SALT_SIZE]);

        let mut nonce = [0u8; NONCE_SIZE];
        nonce.copy_from_slice(&bytes[1 + SALT_SIZE..Self::SIZE]);

        Ok(Self {
            version,
            salt,
            nonce,
        })
    }
}

/// Encrypt data and prepend vault header.
///
/// # Format
///
/// ```text
/// [version: 1 byte][salt: 16 bytes][nonce: 12 bytes][ciphertext + tag]
/// ```
///
/// # Arguments
///
/// * `passphrase` - User passphrase for key derivation
/// * `plaintext` - Data to encrypt
///
/// # Returns
///
/// Complete encrypted vault blob ready for storage.
pub fn seal_vault(passphrase: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let salt = generate_salt();
    let nonce = generate_nonce();
    let key = derive_key(passphrase, &salt)?;

    let ciphertext = encrypt(&key, &nonce, plaintext)?;

    let header = VaultHeader {
        version: VAULT_VERSION,
        salt,
        nonce,
    };

    let mut vault = Vec::with_capacity(VaultHeader::SIZE + ciphertext.len());
    vault.extend_from_slice(&header.to_bytes());
    vault.extend_from_slice(&ciphertext);

    Ok(vault)
}

/// Decrypt a sealed vault.
///
/// # Arguments
///
/// * `passphrase` - User passphrase for key derivation
/// * `vault_data` - Complete encrypted vault blob (header + ciphertext)
///
/// # Returns
///
/// Decrypted plaintext.
///
/// # Errors
///
/// Returns `DecryptionFailed` if passphrase is wrong or data is corrupted.
pub fn open_vault(passphrase: &[u8], vault_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let header = VaultHeader::from_bytes(vault_data)?;
    let ciphertext = &vault_data[VaultHeader::SIZE..];

    let key = derive_key(passphrase, &header.salt)?;
    decrypt(&key, &header.nonce, ciphertext)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Use faster Argon2 params for tests
    fn derive_test_key(passphrase: &[u8], salt: &[u8; SALT_SIZE]) -> Result<SecureKey, CryptoError> {
        derive_key_with_params(passphrase, salt, 1024, 1, 1) // 1 MiB, 1 iter
    }

    #[test]
    fn test_key_derivation_consistency() {
        let passphrase = b"test passphrase";
        let salt = generate_salt();

        let key1 = derive_test_key(passphrase, &salt).unwrap();
        let key2 = derive_test_key(passphrase, &salt).unwrap();

        assert_eq!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_different_salt_produces_different_key() {
        let passphrase = b"test passphrase";
        let salt1 = generate_salt();
        let salt2 = generate_salt();

        let key1 = derive_test_key(passphrase, &salt1).unwrap();
        let key2 = derive_test_key(passphrase, &salt2).unwrap();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = generate_random_key();
        let nonce = generate_nonce();
        let plaintext = b"Hello, secure world!";

        let ciphertext = encrypt(&key, &nonce, plaintext).unwrap();
        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_key_fails_decryption() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        let nonce = generate_nonce();
        let plaintext = b"Secret data";

        let ciphertext = encrypt(&key1, &nonce, plaintext).unwrap();
        let result = decrypt(&key2, &nonce, &ciphertext);

        assert!(matches!(result, Err(CryptoError::DecryptionFailed(_))));
    }

    #[test]
    fn test_wrong_nonce_fails_decryption() {
        let key = generate_random_key();
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();
        let plaintext = b"Secret data";

        let ciphertext = encrypt(&key, &nonce1, plaintext).unwrap();
        let result = decrypt(&key, &nonce2, &ciphertext);

        assert!(matches!(result, Err(CryptoError::DecryptionFailed(_))));
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = generate_random_key();
        let nonce = generate_nonce();
        let plaintext = b"Secret data";

        let mut ciphertext = encrypt(&key, &nonce, plaintext).unwrap();
        // Tamper with ciphertext
        ciphertext[0] ^= 0xFF;

        let result = decrypt(&key, &nonce, &ciphertext);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed(_))));
    }

    #[test]
    fn test_vault_header_serialization() {
        let header = VaultHeader {
            version: VAULT_VERSION,
            salt: generate_salt(),
            nonce: generate_nonce(),
        };

        let bytes = header.to_bytes();
        let parsed = VaultHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.version, header.version);
        assert_eq!(parsed.salt, header.salt);
        assert_eq!(parsed.nonce, header.nonce);
    }

    #[test]
    fn test_seal_open_vault_roundtrip() {
        // Note: This test uses full Argon2 params, may be slow
        // For CI, consider mocking or using test params
        let passphrase = b"wallet-passphrase-2024";
        let plaintext = b"VM state snapshot data here...";

        let vault = seal_vault(passphrase, plaintext).unwrap();
        let decrypted = open_vault(passphrase, &vault).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_passphrase_fails_open() {
        let correct_passphrase = b"correct-passphrase";
        let wrong_passphrase = b"wrong-passphrase";
        let plaintext = b"Secret state";

        let vault = seal_vault(correct_passphrase, plaintext).unwrap();
        let result = open_vault(wrong_passphrase, &vault);

        assert!(matches!(result, Err(CryptoError::DecryptionFailed(_))));
    }

    #[test]
    fn test_secure_key_debug_redacted() {
        let key = generate_random_key();
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains("0x")); // No hex dump
    }
}
