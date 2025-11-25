pub mod bus;
pub mod cpu;
pub mod decoder;
pub mod csr;
pub mod mmu;
pub mod dram;
pub mod clint;
pub mod plic;
pub mod uart;
pub mod virtio;
pub mod emulator;
pub mod console;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Trap {
    InstructionAddressMisaligned(u64),
    InstructionAccessFault(u64),
    IllegalInstruction(u64),
    Breakpoint,
    LoadAddressMisaligned(u64),
    LoadAccessFault(u64),
    StoreAddressMisaligned(u64),
    StoreAccessFault(u64),
    EnvironmentCallFromU,
    EnvironmentCallFromS,
    EnvironmentCallFromM,
    InstructionPageFault(u64),
    LoadPageFault(u64),
    StorePageFault(u64),
    
    MachineSoftwareInterrupt,
    MachineTimerInterrupt,
    MachineExternalInterrupt,
    SupervisorSoftwareInterrupt,
    SupervisorTimerInterrupt,
    SupervisorExternalInterrupt,

    // Custom internal errors
    RequestedTrap(u64), // For testing (software interrupts, etc)
    Fatal(String),
}

impl std::fmt::Display for Trap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Trap {}
