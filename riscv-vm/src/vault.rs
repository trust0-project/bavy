//! Secure vault storage for encrypted VM state persistence.
//!
//! This module provides file-based encrypted storage for Node.js and native
//! platforms. It uses the crypto module for AES-256-GCM encryption.
//!
//! ## Features
//!
//! - File-based encrypted storage
//! - Cross-platform (Linux, macOS, Windows)
//! - Atomic writes to prevent corruption
//! - Version-aware format for migrations
//!
//! ## Security Model
//!
//! - Vault file is always encrypted at rest
//! - Passphrase never stored - only used to derive key
//! - Wrong passphrase = decryption failure (no data leak)

use crate::crypto::{open_vault, seal_vault, CryptoError};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Default vault filename
pub const VAULT_FILENAME: &str = "havy-vault.enc";

/// Default vault directory (relative to home or current dir)
pub const VAULT_DIRECTORY: &str = ".havy-os";

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Vault operation errors.
#[derive(Debug)]
pub enum VaultError {
    /// Vault file not found.
    NotFound,
    /// Decryption failed (wrong passphrase or corrupted data).
    DecryptionFailed(String),
    /// Storage I/O error.
    IoError(io::Error),
    /// Cryptographic error.
    CryptoError(CryptoError),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::NotFound => write!(f, "Vault not found"),
            VaultError::DecryptionFailed(msg) => {
                write!(f, "Decryption failed: {}. Check your passphrase.", msg)
            }
            VaultError::IoError(e) => write!(f, "I/O error: {}", e),
            VaultError::CryptoError(e) => write!(f, "Crypto error: {}", e),
        }
    }
}

impl std::error::Error for VaultError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VaultError::IoError(e) => Some(e),
            VaultError::CryptoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for VaultError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::NotFound {
            VaultError::NotFound
        } else {
            VaultError::IoError(e)
        }
    }
}

impl From<CryptoError> for VaultError {
    fn from(e: CryptoError) -> Self {
        match e {
            CryptoError::DecryptionFailed(msg) => VaultError::DecryptionFailed(msg),
            other => VaultError::CryptoError(other),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VAULT TRAIT
// ═══════════════════════════════════════════════════════════════════════════════

/// Secure vault interface for encrypted storage.
pub trait SecureVault {
    /// Check if the vault exists.
    fn exists(&self) -> bool;

    /// Save encrypted state.
    fn save(&self, passphrase: &[u8], data: &[u8]) -> Result<(), VaultError>;

    /// Load and decrypt state.
    fn load(&self, passphrase: &[u8]) -> Result<Vec<u8>, VaultError>;

    /// Permanently delete the vault.
    fn destroy(&self) -> Result<(), VaultError>;
}

// ═══════════════════════════════════════════════════════════════════════════════
// FILE VAULT IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/// File-based secure vault for Node.js and native platforms.
#[derive(Debug, Clone)]
pub struct FileVault {
    /// Path to the vault file.
    path: PathBuf,
}

impl FileVault {
    /// Create a new file vault at the specified path.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Create a vault in the default location.
    ///
    /// Uses `~/.havy-os/havy-vault.enc` on Unix systems.
    pub fn default_location() -> Self {
        let base = if let Some(home) = dirs_home() {
            home.join(VAULT_DIRECTORY)
        } else {
            PathBuf::from(VAULT_DIRECTORY)
        };

        Self::new(base.join(VAULT_FILENAME))
    }

    /// Get the path to the vault file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ensure the parent directory exists.
    fn ensure_dir(&self) -> Result<(), io::Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl SecureVault for FileVault {
    fn exists(&self) -> bool {
        self.path.exists()
    }

    fn save(&self, passphrase: &[u8], data: &[u8]) -> Result<(), VaultError> {
        log::info!(
            "[Vault] Saving encrypted state ({} bytes) to {:?}",
            data.len(),
            self.path
        );

        // Ensure directory exists
        self.ensure_dir()?;

        // Encrypt the data
        let encrypted = seal_vault(passphrase, data)?;

        // Write atomically using a temp file
        let temp_path = self.path.with_extension("enc.tmp");

        // Write to temp file
        fs::write(&temp_path, &encrypted)?;

        // Rename to final path (atomic on most filesystems)
        fs::rename(&temp_path, &self.path)?;

        log::info!(
            "[Vault] Saved {} bytes (encrypted) to {:?}",
            encrypted.len(),
            self.path
        );

        Ok(())
    }

    fn load(&self, passphrase: &[u8]) -> Result<Vec<u8>, VaultError> {
        log::info!("[Vault] Loading encrypted state from {:?}", self.path);

        // Read encrypted data
        let encrypted = fs::read(&self.path)?;

        log::info!("[Vault] Read {} bytes (encrypted)", encrypted.len());

        // Decrypt
        let decrypted = open_vault(passphrase, &encrypted)?;

        log::info!("[Vault] Decrypted to {} bytes", decrypted.len());

        Ok(decrypted)
    }

    fn destroy(&self) -> Result<(), VaultError> {
        log::info!("[Vault] Destroying vault at {:?}", self.path);

        match fs::remove_file(&self.path) {
            Ok(()) => {
                log::info!("[Vault] Vault destroyed");
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                log::info!("[Vault] Vault already destroyed or never existed");
                Ok(())
            }
            Err(e) => Err(VaultError::IoError(e)),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Get the user's home directory.
fn dirs_home() -> Option<PathBuf> {
    // Simple cross-platform home directory detection
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Create a unique temp directory for tests
    fn create_test_dir() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("havy-vault-test-{}", timestamp));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_file_vault_save_load() {
        let dir = create_test_dir();
        let vault_path = dir.join("test-vault.enc");
        let vault = FileVault::new(&vault_path);

        let passphrase = b"test-passphrase-2024";
        let data = b"This is my secret VM state!";

        // Initially should not exist
        assert!(!vault.exists());

        // Save
        vault.save(passphrase, data).unwrap();
        assert!(vault.exists());

        // Load
        let loaded = vault.load(passphrase).unwrap();
        assert_eq!(data.as_slice(), loaded.as_slice());

        // Destroy
        vault.destroy().unwrap();
        assert!(!vault.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let dir = create_test_dir();
        let vault_path = dir.join("test-vault.enc");
        let vault = FileVault::new(&vault_path);

        let correct = b"correct-passphrase";
        let wrong = b"wrong-passphrase";
        let data = b"Secret data";

        vault.save(correct, data).unwrap();

        let result = vault.load(wrong);
        assert!(matches!(result, Err(VaultError::DecryptionFailed(_))));

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_vault_not_found() {
        let dir = create_test_dir();
        let vault_path = dir.join("nonexistent.enc");
        let vault = FileVault::new(&vault_path);

        let result = vault.load(b"any-passphrase");
        assert!(matches!(result, Err(VaultError::NotFound)));

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_destroy_nonexistent() {
        let dir = create_test_dir();
        let vault_path = dir.join("nonexistent.enc");
        let vault = FileVault::new(&vault_path);

        // Should not error
        vault.destroy().unwrap();

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }
}

