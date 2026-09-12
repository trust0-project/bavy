//! Allwinner D1 / Lichee RV memory map (same guest binary as silicon).

use super::{MemoryMap, PlicMap, UartMap, DRAM_SIZE};

pub const DRAM_BASE: u64 = 0x4000_0000;
/// Inside the 2 MiB OpenSBI reservation (`0x4000_0000`..`0x4020_0000`).
pub const DTB_OFFSET: u64 = 0x0010_0000;
pub const KERNEL_START: u64 = 0x4020_0000;

pub const UART_BASE: u64 = 0x0250_0000;
pub const UART_SIZE: u64 = 0x400;
pub const UART_IRQ: u32 = 18; // T-Head PLIC UART0

pub const PLIC_BASE: u64 = 0x1000_0000;
pub const PLIC_SIZE: u64 = 0x0400_0000;
/// Phase A: still 31 sources in the emulator; Phase E grows toward ~175.
pub const PLIC_NDEV: u32 = 31;

pub const MMC0_BASE: u64 = 0x0402_0000;
pub const EMAC_BASE: u64 = 0x0450_0000;
pub const DE_MIXER0: u64 = 0x0510_0000;
pub const TCON_LCD0: u64 = 0x0546_1000;
pub const I2C2_BASE: u64 = 0x0250_2000;

/// D1 timebase: 24 MHz oscillator. Only on this machine.
pub const TIMEBASE_HZ: u32 = 24_000_000;

pub static MAP: MemoryMap = MemoryMap {
    dram_base: DRAM_BASE,
    dram_size: DRAM_SIZE,
    dtb_offset: DTB_OFFSET,
    uart: UartMap {
        base: UART_BASE,
        size: UART_SIZE,
        irq: UART_IRQ,
        stride: 4,
        compatible: "snps,dw-apb-uart",
    },
    plic: PlicMap {
        base: PLIC_BASE,
        size: PLIC_SIZE,
        ndev: PLIC_NDEV,
        compatible: "thead,c900-plic",
    },
    clint: None,
    virtio: None,
    timebase_hz: TIMEBASE_HZ,
    compatible: &["allwinner,sun20i-d1"],
    model: "lichee-rv",
};
