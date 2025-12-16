use crate::csr::Mode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Version identifier for snapshot compatibility checks.
pub const SNAPSHOT_VERSION: &str = "2.0";

/// Full emulator snapshot including CPU, devices and DRAM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: String,
    pub cpu: CpuSnapshot,
    pub devices: DeviceSnapshot,
    pub memory: Vec<MemRegionSnapshot>,
}

/// Serializable CPU state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub pc: u64,
    pub mode: Mode,
    pub regs: [u64; 32],
    pub csrs: HashMap<u16, u64>,
}

/// Serializable device state bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub clint: ClintSnapshot,
    pub plic: PlicSnapshot,
    pub uart: UartSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClintSnapshot {
    pub msip: Vec<u32>,
    pub mtime: u64,
    pub mtimecmp: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlicSnapshot {
    pub priority: Vec<u32>,
    pub pending: u32,
    pub enable: Vec<u32>,
    pub threshold: Vec<u32>,
    pub active: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UartSnapshot {
    pub rx_fifo: Vec<u8>,
    pub tx_fifo: Vec<u8>,
    pub ier: u8,
    pub iir: u8,
    pub fcr: u8,
    pub lcr: u8,
    pub mcr: u8,
    pub lsr: u8,
    pub msr: u8,
    pub scr: u8,
    pub dll: u8,
    pub dlm: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemRegionSnapshot {
    pub base: u64,
    pub size: u64,
    pub hash: String,
    pub data: Option<Vec<u8>>,
}






