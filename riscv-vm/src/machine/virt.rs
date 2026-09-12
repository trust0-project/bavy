//! QEMU `virt`-shaped memory map (portable browser/desktop VM).

use super::{ClintMap, MemoryMap, PlicMap, UartMap, VirtioMap, DRAM_SIZE};

pub const DRAM_BASE: u64 = 0x8000_0000;
/// 32 MiB into DRAM (historical `0x8200_0000`). Not 2 MiB — keep boot stable.
pub const DTB_OFFSET: u64 = 0x0200_0000;

pub const UART_BASE: u64 = 0x1000_0000;
pub const UART_SIZE: u64 = 0x100;
pub const UART_IRQ: u32 = 10;

pub const PLIC_BASE: u64 = 0x0C00_0000;
/// Emulator window (64 MiB). DTB `reg` size is the smaller QEMU-virt 0x600000.
pub const PLIC_SIZE: u64 = 0x400_0000;
pub const PLIC_DTB_SIZE: u64 = 0x60_0000;
/// Match `devices/plic.rs` `NUM_SOURCES - 1`. Never advertise 127 while only 32 exist.
pub const PLIC_NDEV: u32 = 31;

pub const CLINT_BASE: u64 = 0x0200_0000;
pub const CLINT_SIZE: u64 = 0x10000;

pub const VIRTIO_BASE: u64 = 0x1000_1000;
pub const VIRTIO_STRIDE: u64 = 0x1000;
pub const VIRTIO_MAX_SLOTS: usize = 6;

/// Virt timebase: 10 MHz (QEMU virt). Documented; do not use D1's 24 MHz here.
pub const TIMEBASE_HZ: u32 = 10_000_000;

pub static MAP: MemoryMap = MemoryMap {
    dram_base: DRAM_BASE,
    dram_size: DRAM_SIZE,
    dtb_offset: DTB_OFFSET,
    uart: UartMap {
        base: UART_BASE,
        size: UART_SIZE,
        irq: UART_IRQ,
        stride: 1,
        compatible: "ns16550a",
    },
    plic: PlicMap {
        base: PLIC_BASE,
        size: PLIC_SIZE,
        ndev: PLIC_NDEV,
        compatible: "riscv,plic0",
    },
    clint: Some(ClintMap {
        base: CLINT_BASE,
        size: CLINT_SIZE,
    }),
    virtio: Some(VirtioMap {
        base: VIRTIO_BASE,
        stride: VIRTIO_STRIDE,
        max_slots: VIRTIO_MAX_SLOTS,
    }),
    timebase_hz: TIMEBASE_HZ,
    compatible: &["riscv-virtio,qemu", "riscv-virtio"],
    model: "riscv-virt",
};
