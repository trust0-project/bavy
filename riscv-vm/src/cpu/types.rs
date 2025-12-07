use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    User,
    Supervisor,
    Machine,
}

impl Mode {
    /// Encode privilege mode into the MPP/SPP field encoding.
    pub fn to_mpp(self) -> u64 {
        match self {
            Mode::User => 0b00,
            Mode::Supervisor => 0b01,
            Mode::Machine => 0b11,
        }
    }

    /// Decode MPP/SPP field into a privilege mode.
    pub fn from_mpp(bits: u64) -> Mode {
        match bits & 0b11 {
            0b00 => Mode::User,
            0b01 => Mode::Supervisor,
            // 0b10 is reserved; treat as Machine for WARL coercion.
            _ => Mode::Machine,
        }
    }

    /// Numeric privilege level used in CSR access bounds checks.
    pub fn privilege_level(self) -> u16 {
        match self {
            Mode::User => 0,
            Mode::Supervisor => 1,
            Mode::Machine => 3,
        }
    }
}

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

    // Sleep request
    Wfi,

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
