//! Shared machine identity for the RISC-V emulator.
//!
//! The CPU, MMU, block engine, SBI dispatcher, DRAM, and hart registry are
//! **machine-agnostic**. Board identity lives here: physical map, DTB strings,
//! timebase, and hart policy.
//!
//! ```text
//! Shared (do not put Machine on Cpu):
//!   cpu/  engine/  jit/  mmu.rs  dram.rs (base is a field)
//!   sbi/* call convention   hart_registry
//!
//! Machine-owned:
//!   MemoryMap  DTB  which devices are constructed  UART/PLIC/CLINT bases
//! ```
//!
//! Two machines, never one frankenstein DTB:
//! - [`Machine::Virt`] — QEMU-virt shaped (browser/desktop, optional Linux later)
//! - [`Machine::D1`] — Allwinner D1 / Lichee RV (same kernel binary as silicon)
//!
//! Phase A: identity only. Phase B+ correctness/JIT/devices hang off this map.

pub mod d1;
pub mod virt;

/// Guest ISA string; must match `misa` (`RV64IMAFDC` + Zicsr + Zifencei).
pub const ISA_STRING: &str = "rv64imafdc_zicsr_zifencei";

/// Default DRAM size on every target (native, Wasm browser, Wasm Node).
pub const DRAM_SIZE: usize = 512 * 1024 * 1024;

/// Board identity. Selects the guest physical map, DTB, and device set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Machine {
    /// QEMU `virt`-shaped: DRAM `0x8000_0000`, NS16550, SiFive PLIC+CLINT.
    Virt,
    /// Allwinner D1: DRAM `0x4000_0000`, DW UART, T-Head PLIC, SBI TIME.
    D1,
}

impl Machine {
    /// Parse CLI / JS `--machine` values (`virt`, `d1`, `sun20i-d1`, `qemu`).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "virt" | "qemu" | "qemu-virt" | "riscv-virtio" => Some(Self::Virt),
            "d1" | "sun20i-d1" | "allwinner,sun20i-d1" | "lichee" | "lichee-rv" => {
                Some(Self::D1)
            }
            _ => None,
        }
    }

    /// Infer identity from a DRAM base. Unknown bases are not a machine.
    pub fn from_dram_base(base: u64) -> Option<Self> {
        if base == virt::DRAM_BASE {
            Some(Self::Virt)
        } else if base == d1::DRAM_BASE {
            Some(Self::D1)
        } else {
            None
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Virt => "virt",
            Self::D1 => "d1",
        }
    }

    pub fn memory_map(self) -> &'static MemoryMap {
        match self {
            Self::Virt => &virt::MAP,
            Self::D1 => &d1::MAP,
        }
    }

    /// Root DTB `compatible` (first string). Virt never claims `sun20i-d1`.
    pub fn compatible(self) -> &'static str {
        self.memory_map().compatible[0]
    }

    pub fn model(self) -> &'static str {
        self.memory_map().model
    }

    pub fn timebase_hz(self) -> u32 {
        self.memory_map().timebase_hz
    }

    /// Hart count advertised in the DTB / CLINT.
    ///
    /// Virt: 1–N host threads. D1 hardware is uniprocessor; the VM still
    /// emulates 1 unless the caller explicitly asks for more (emulator-only).
    /// Wasm without SharedArrayBuffer must be 1 regardless of request.
    pub fn clamp_harts(self, requested: usize, sab_available: bool) -> usize {
        let n = requested.max(1);
        if !sab_available {
            return 1;
        }
        match self {
            Self::Virt => n,
            Self::D1 => n.min(1).max(1),
        }
    }

    /// Native CLI default when `--harts 0`.
    pub fn default_harts_native(self, host_cpus: usize) -> usize {
        match self {
            Self::Virt => host_cpus.max(1),
            Self::D1 => 1,
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self::Virt
    }
}

impl core::fmt::Display for Machine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// UART programming model on this board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UartMap {
    pub base: u64,
    pub size: u64,
    pub irq: u32,
    /// Guest register stride: 1 for NS16550A (byte), 4 for DesignWare APB.
    pub stride: u32,
    pub compatible: &'static str,
}

impl UartMap {
    /// Convert a byte MMIO offset into the 16550 register index (0..=7).
    #[inline]
    pub fn reg_index(self, mmio_offset: u64) -> u64 {
        mmio_offset / self.stride as u64
    }
}

/// PLIC programming model on this board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlicMap {
    pub base: u64,
    pub size: u64,
    /// `riscv,ndev` — must not exceed the emulator's source count.
    pub ndev: u32,
    pub compatible: &'static str,
}

/// Optional SiFive CLINT. `None` on D1 (timer via SBI TIME, IPI via SBI).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClintMap {
    pub base: u64,
    pub size: u64,
}

/// VirtIO MMIO window. `None` on D1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtioMap {
    pub base: u64,
    pub stride: u64,
    pub max_slots: usize,
}

/// Guest-physical identity for one [`Machine`].
#[derive(Clone, Copy, Debug)]
pub struct MemoryMap {
    pub dram_base: u64,
    pub dram_size: usize,
    /// DTB physical address = `dram_base + dtb_offset`.
    pub dtb_offset: u64,
    pub uart: UartMap,
    pub plic: PlicMap,
    pub clint: Option<ClintMap>,
    pub virtio: Option<VirtioMap>,
    pub timebase_hz: u32,
    pub compatible: &'static [&'static str],
    pub model: &'static str,
}

impl MemoryMap {
    #[inline]
    pub fn dtb_addr(&self) -> u64 {
        self.dram_base + self.dtb_offset
    }

    #[inline]
    pub fn dram_end(&self) -> u64 {
        self.dram_base + self.dram_size as u64
    }
}

/// Devices actually attached at DTB generation time (not compile-time `has_*`).
#[derive(Clone, Debug, Default)]
pub struct AttachedDevices {
    pub has_display: bool,
    pub has_mmc: bool,
    pub has_emac: bool,
    pub has_touch: bool,
    pub has_audio: bool,
    /// Instantiated virtio-mmio slots (Virt only). Never advertise empty slots.
    pub virtio_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virt_and_d1_maps_do_not_share_dram() {
        assert_ne!(Machine::Virt.memory_map().dram_base, Machine::D1.memory_map().dram_base);
        assert_eq!(Machine::Virt.compatible(), "riscv-virtio,qemu");
        assert_eq!(Machine::D1.compatible(), "allwinner,sun20i-d1");
    }

    #[test]
    fn isa_string_includes_fd() {
        assert!(ISA_STRING.contains("imafdc"));
        assert!(ISA_STRING.contains("zicsr"));
        assert!(ISA_STRING.contains("zifencei"));
    }

    #[test]
    fn wasm_without_sab_is_uniprocessor() {
        assert_eq!(Machine::Virt.clamp_harts(8, false), 1);
        assert_eq!(Machine::Virt.clamp_harts(4, true), 4);
        assert_eq!(Machine::D1.clamp_harts(8, true), 1);
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(Machine::parse("virt"), Some(Machine::Virt));
        assert_eq!(Machine::parse("D1"), Some(Machine::D1));
        assert_eq!(Machine::parse("nope"), None);
    }

    #[test]
    fn timebases() {
        assert_eq!(Machine::Virt.timebase_hz(), 10_000_000);
        assert_eq!(Machine::D1.timebase_hz(), 24_000_000);
    }
}
