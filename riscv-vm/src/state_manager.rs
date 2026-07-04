//! VM State Manager - Orchestrates encrypted state persistence.
//!
//! This module connects the snapshot serialization with the vault encryption
//! to provide a complete solution for saving and restoring encrypted VM state.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use riscv_vm::state_manager::{StateManager, StateManagerError};
//! use riscv_vm::vault::FileVault;
//!
//! // Create state manager with file vault
//! let vault = FileVault::default_location();
//! let manager = StateManager::new(vault);
//!
//! // Check if state exists
//! if manager.has_state() {
//!     // Restore existing state
//!     let snapshot = manager.unlock(b"user-passphrase")?;
//! } else {
//!     // First boot - initialize new state
//!     manager.initialize(b"user-passphrase")?;
//! }
//!
//! // Save current state
//! manager.save_state(&snapshot, b"user-passphrase")?;
//! ```

use crate::snapshot::Snapshot;
use crate::vault::{SecureVault, VaultError};

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// State manager errors.
#[derive(Debug)]
pub enum StateError {
    /// Vault operation failed.
    VaultError(VaultError),
    /// Serialization failed.
    SerializationError(String),
    /// Deserialization failed (corrupted state).
    DeserializationError(String),
    /// State already exists (for initialize).
    StateAlreadyExists,
    /// State does not exist (for unlock).
    StateNotFound,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::VaultError(e) => write!(f, "Vault error: {}", e),
            StateError::SerializationError(msg) => write!(f, "Serialization failed: {}", msg),
            StateError::DeserializationError(msg) => write!(f, "Deserialization failed: {}", msg),
            StateError::StateAlreadyExists => write!(f, "State already exists"),
            StateError::StateNotFound => write!(f, "State not found"),
        }
    }
}

impl std::error::Error for StateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StateError::VaultError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VaultError> for StateError {
    fn from(e: VaultError) -> Self {
        match e {
            VaultError::NotFound => StateError::StateNotFound,
            other => StateError::VaultError(other),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE MANAGER
// ═══════════════════════════════════════════════════════════════════════════════

/// Manages encrypted VM state persistence.
///
/// The StateManager orchestrates:
/// - Snapshot serialization (bincode)
/// - Encryption (via vault)
/// - Storage (via vault)
pub struct StateManager<V: SecureVault> {
    vault: V,
}

impl<V: SecureVault> StateManager<V> {
    /// Create a new state manager with the given vault.
    pub fn new(vault: V) -> Self {
        Self { vault }
    }

    /// Check if encrypted state exists.
    pub fn has_state(&self) -> bool {
        self.vault.exists()
    }

    /// Initialize a new vault with an empty/default state.
    ///
    /// This is called on first boot when no state exists.
    /// Creates an empty snapshot and encrypts it with the provided passphrase.
    ///
    /// # Errors
    ///
    /// Returns `StateAlreadyExists` if a vault already exists.
    pub fn initialize(&self, passphrase: &[u8]) -> Result<Snapshot, StateError> {
        if self.vault.exists() {
            return Err(StateError::StateAlreadyExists);
        }

        log::info!("[StateManager] Initializing new vault...");

        // Create default/empty snapshot
        let snapshot = Snapshot::default();

        // Save it
        self.save_state(&snapshot, passphrase)?;

        log::info!("[StateManager] Vault initialized successfully");
        Ok(snapshot)
    }

    /// Unlock an existing vault and return the decrypted snapshot.
    ///
    /// # Errors
    ///
    /// Returns `StateNotFound` if no vault exists.
    /// Returns `VaultError::DecryptionFailed` if passphrase is wrong.
    pub fn unlock(&self, passphrase: &[u8]) -> Result<Snapshot, StateError> {
        if !self.vault.exists() {
            return Err(StateError::StateNotFound);
        }

        log::info!("[StateManager] Unlocking vault...");

        // Load and decrypt
        let data = self.vault.load(passphrase)?;

        // Deserialize
        let snapshot: Snapshot = bincode::deserialize(&data)
            .map_err(|e| StateError::DeserializationError(e.to_string()))?;

        log::info!(
            "[StateManager] Vault unlocked, snapshot version: {}",
            snapshot.version
        );
        Ok(snapshot)
    }

    /// Save the current VM state to the vault.
    ///
    /// This encrypts the snapshot with the provided passphrase and stores it.
    pub fn save_state(&self, snapshot: &Snapshot, passphrase: &[u8]) -> Result<(), StateError> {
        log::info!("[StateManager] Saving state to vault...");

        // Serialize
        let data = bincode::serialize(snapshot)
            .map_err(|e| StateError::SerializationError(e.to_string()))?;

        log::info!("[StateManager] Serialized snapshot: {} bytes", data.len());

        // Encrypt and save
        self.vault.save(passphrase, &data)?;

        log::info!("[StateManager] State saved successfully");
        Ok(())
    }

    /// Destroy the vault (irreversible).
    ///
    /// Use with caution! This permanently deletes all encrypted state.
    pub fn destroy(&self) -> Result<(), StateError> {
        log::warn!("[StateManager] Destroying vault...");
        self.vault.destroy()?;
        log::info!("[StateManager] Vault destroyed");
        Ok(())
    }

    /// Get a reference to the underlying vault.
    pub fn vault(&self) -> &V {
        &self.vault
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DEFAULT SNAPSHOT
// ═══════════════════════════════════════════════════════════════════════════════

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            version: crate::snapshot::SNAPSHOT_VERSION.to_string(),
            cpu: crate::snapshot::CpuSnapshot {
                pc: 0,
                mode: crate::csr::Mode::Machine,
                regs: [0u64; 32],
                fregs: [0u64; 32],
                csrs: std::collections::HashMap::new(),
            },
            devices: crate::snapshot::DeviceSnapshot {
                clint: crate::snapshot::ClintSnapshot {
                    msip: vec![],
                    mtime: 0,
                    mtimecmp: vec![],
                },
                plic: crate::snapshot::PlicSnapshot {
                    priority: vec![],
                    pending: 0,
                    enable: vec![],
                    threshold: vec![],
                    active: vec![],
                },
                uart: crate::snapshot::UartSnapshot {
                    rx_fifo: vec![],
                    tx_fifo: vec![],
                    ier: 0,
                    iir: 0,
                    fcr: 0,
                    lcr: 0,
                    mcr: 0,
                    lsr: 0,
                    msr: 0,
                    scr: 0,
                    dll: 0,
                    dlm: 0,
                },
            },
            memory: vec![],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::FileVault;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_dir() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("havy-state-test-{}", timestamp));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_state_manager_lifecycle() {
        let dir = create_test_dir();
        let vault = FileVault::new(dir.join("test-vault.enc"));
        let manager = StateManager::new(vault);

        let passphrase = b"test-passphrase-2024";

        // Initially no state
        assert!(!manager.has_state());

        // Initialize
        let snapshot = manager.initialize(passphrase).unwrap();
        assert!(manager.has_state());
        assert_eq!(snapshot.version, crate::snapshot::SNAPSHOT_VERSION);

        // Unlock
        let restored = manager.unlock(passphrase).unwrap();
        assert_eq!(restored.version, snapshot.version);

        // Destroy
        manager.destroy().unwrap();
        assert!(!manager.has_state());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_initialize_already_exists() {
        let dir = create_test_dir();
        let vault = FileVault::new(dir.join("test-vault.enc"));
        let manager = StateManager::new(vault);

        let passphrase = b"test-passphrase";

        manager.initialize(passphrase).unwrap();

        // Second initialize should fail
        let result = manager.initialize(passphrase);
        assert!(matches!(result, Err(StateError::StateAlreadyExists)));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_unlock_wrong_passphrase() {
        let dir = create_test_dir();
        let vault = FileVault::new(dir.join("test-vault.enc"));
        let manager = StateManager::new(vault);

        manager.initialize(b"correct-passphrase").unwrap();

        let result = manager.unlock(b"wrong-passphrase");
        assert!(matches!(
            result,
            Err(StateError::VaultError(VaultError::DecryptionFailed(_)))
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_unlock_not_found() {
        let dir = create_test_dir();
        let vault = FileVault::new(dir.join("nonexistent.enc"));
        let manager = StateManager::new(vault);

        let result = manager.unlock(b"any-passphrase");
        assert!(matches!(result, Err(StateError::StateNotFound)));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_and_restore_modified_state() {
        let dir = create_test_dir();
        let vault = FileVault::new(dir.join("test-vault.enc"));
        let manager = StateManager::new(vault);

        let passphrase = b"test-passphrase";

        // Initialize
        let mut snapshot = manager.initialize(passphrase).unwrap();

        // Modify state
        snapshot.cpu.pc = 0x8000_0000;
        snapshot.cpu.regs[1] = 42;

        // Save modified state
        manager.save_state(&snapshot, passphrase).unwrap();

        // Restore and verify
        let restored = manager.unlock(passphrase).unwrap();
        assert_eq!(restored.cpu.pc, 0x8000_0000);
        assert_eq!(restored.cpu.regs[1], 42);

        let _ = fs::remove_dir_all(&dir);
    }
}
