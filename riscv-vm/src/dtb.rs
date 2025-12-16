//! Device Tree Blob (DTB) generation for OpenSBI compliance.
//!
//! This module generates a minimal Flattened Device Tree (FDT) that describes
//! the VM's hardware to the kernel. On real hardware, OpenSBI passes the DTB
//! address in `a1`. This module replicates that behavior for the emulator.
//!
//! ## DTB Memory Layout
//!
//! The DTB is stored at a fixed location in DRAM (just below the kernel):
//! - DTB Address: 0x8200_0000 (2MB after DRAM start, before kernel at 0x8020_0000)
//! - Max Size: 64KB
//!
//! ## OpenSBI Boot Protocol
//!
//! When OpenSBI transfers control to S-mode kernel:
//! - a0 = hartid (hardware thread ID)
//! - a1 = DTB physical address (8-byte aligned)

use crate::dram::Dram;

/// DTB location in DRAM (2MB from start, leaves room before typical kernel load)
pub const DTB_ADDRESS: u64 = 0x8200_0000;

/// Maximum DTB size
pub const DTB_MAX_SIZE: usize = 64 * 1024;

/// FDT header magic number
const FDT_MAGIC: u32 = 0xd00dfeed;

/// FDT version (17 is common)
const FDT_VERSION: u32 = 17;

/// FDT last compatible version
const FDT_LAST_COMP_VERSION: u32 = 16;

/// FDT tokens
const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_END: u32 = 0x00000009;

/// D1 device configuration for DTB generation
#[derive(Default, Clone)]
pub struct D1DeviceConfig {
    /// D1 Display Engine (DE2 + TCON)
    pub has_display: bool,
    /// D1 MMC/SD controller
    pub has_mmc: bool,
    /// D1 EMAC Ethernet controller
    pub has_emac: bool,
    /// D1 I2C2 + GT911 touchscreen
    pub has_touch: bool,
}

/// Generate a minimal DTB for the RISC-V VM.
///
/// # Arguments
/// * `num_harts` - Number of CPU harts
/// * `memory_size` - Total DRAM size in bytes
/// * `d1_config` - D1 device configuration
///
/// # Returns
/// A Vec<u8> containing the complete DTB blob.
pub fn generate_dtb(
    num_harts: usize,
    memory_size: u64,
    d1_config: &D1DeviceConfig,
) -> Vec<u8> {
    let mut builder = DtbBuilder::new();
    
    // Root node (/)
    builder.begin_node("");
    builder.add_prop_u32("#address-cells", 2);
    builder.add_prop_u32("#size-cells", 2);
    builder.add_prop_string("compatible", "allwinner,sun20i-d1");
    builder.add_prop_string("model", "riscv-vm-d1");
    
    // /chosen - kernel command line and console
    builder.begin_node("chosen");
    builder.add_prop_string("bootargs", "earlycon=sbi console=ttyS0");
    builder.add_prop_string("stdout-path", "/soc/serial@10000000");
    builder.end_node();
    
    // /cpus
    builder.begin_node("cpus");
    builder.add_prop_u32("#address-cells", 1);
    builder.add_prop_u32("#size-cells", 0);
    builder.add_prop_u32("timebase-frequency", 10_000_000); // 10 MHz
    
    for hart in 0..num_harts {
        builder.begin_node(&format!("cpu@{}", hart));
        builder.add_prop_string("device_type", "cpu");
        builder.add_prop_u32("reg", hart as u32);
        builder.add_prop_string("status", "okay");
        builder.add_prop_string("compatible", "riscv");
        builder.add_prop_string("riscv,isa", "rv64imac_zicsr_zifencei");
        builder.add_prop_string("mmu-type", "riscv,sv39");
        
        // CPU interrupt controller
        builder.begin_node("interrupt-controller");
        builder.add_prop_u32("#interrupt-cells", 1);
        builder.add_prop_empty("interrupt-controller");
        builder.add_prop_string("compatible", "riscv,cpu-intc");
        builder.add_prop_u32("phandle", (hart + 1) as u32);
        builder.end_node();
        
        builder.end_node();
    }
    builder.end_node(); // /cpus
    
    // /memory@80000000
    builder.begin_node("memory@80000000");
    builder.add_prop_string("device_type", "memory");
    builder.add_prop_reg64(0x8000_0000, memory_size);
    builder.end_node();
    
    // /soc
    builder.begin_node("soc");
    builder.add_prop_u32("#address-cells", 2);
    builder.add_prop_u32("#size-cells", 2);
    builder.add_prop_string("compatible", "simple-bus");
    builder.add_prop_empty("ranges");
    
    // CLINT @ 0x0200_0000
    builder.begin_node("clint@2000000");
    builder.add_prop_string("compatible", "riscv,clint0");
    builder.add_prop_reg64(0x0200_0000, 0x10000);
    // interrupts-extended: list of (phandle, irq) pairs for each hart
    let mut clint_ints = Vec::new();
    for hart in 0..num_harts {
        clint_ints.push((hart + 1) as u32); // phandle
        clint_ints.push(3); // MSI
        clint_ints.push((hart + 1) as u32); // phandle
        clint_ints.push(7); // MTI
    }
    builder.add_prop_u32_array("interrupts-extended", &clint_ints);
    builder.end_node();
    
    // PLIC @ 0x0C00_0000
    builder.begin_node("interrupt-controller@c000000");
    builder.add_prop_string("compatible", "riscv,plic0");
    builder.add_prop_u32("#interrupt-cells", 1);
    builder.add_prop_empty("interrupt-controller");
    builder.add_prop_reg64(0x0C00_0000, 0x600000);
    builder.add_prop_u32("riscv,ndev", 127);
    builder.add_prop_u32("phandle", 100);
    // interrupts-extended for PLIC
    let mut plic_ints = Vec::new();
    for hart in 0..num_harts {
        plic_ints.push((hart + 1) as u32); // phandle
        plic_ints.push(9); // S-mode external interrupt
        plic_ints.push((hart + 1) as u32);
        plic_ints.push(11); // M-mode external interrupt
    }
    builder.add_prop_u32_array("interrupts-extended", &plic_ints);
    builder.end_node();
    
    // UART @ 0x1000_0000
    builder.begin_node("serial@10000000");
    builder.add_prop_string("compatible", "ns16550a");
    builder.add_prop_reg64(0x1000_0000, 0x100);
    builder.add_prop_u32("clock-frequency", 3686400);
    builder.add_prop_u32("interrupts", 10);
    builder.add_prop_u32("interrupt-parent", 100); // PLIC phandle
    builder.end_node();
    
    // D1 Display Engine @ 0x0510_0000 (DE2) + 0x0546_1000 (TCON)
    if d1_config.has_display {
        builder.begin_node("display-engine@5100000");
        builder.add_prop_string("compatible", "allwinner,sun20i-d1-de2");
        builder.add_prop_reg64(0x0510_0000, 0x10000); // DE2 mixer
        builder.add_prop_u32("interrupts", 42); // DE2 interrupt
        builder.add_prop_u32("interrupt-parent", 100);
        builder.add_prop_string("status", "okay");
        builder.end_node();
        
        builder.begin_node("lcd-controller@5461000");
        builder.add_prop_string("compatible", "allwinner,sun20i-d1-tcon-lcd");
        builder.add_prop_reg64(0x0546_1000, 0x1000); // TCON-LCD0
        builder.add_prop_u32("interrupts", 106); // TCON interrupt
        builder.add_prop_u32("interrupt-parent", 100);
        builder.add_prop_string("status", "okay");
        builder.end_node();
    }
    
    // D1 MMC @ 0x0402_0000
    if d1_config.has_mmc {
        builder.begin_node("mmc@4020000");
        builder.add_prop_string("compatible", "allwinner,sun20i-d1-mmc");
        builder.add_prop_reg64(0x0402_0000, 0x1000);
        builder.add_prop_u32("interrupts", 56); // MMC0 interrupt
        builder.add_prop_u32("interrupt-parent", 100);
        builder.add_prop_string("status", "okay");
        builder.end_node();
    }
    
    // D1 EMAC @ 0x0450_0000
    if d1_config.has_emac {
        builder.begin_node("ethernet@4500000");
        builder.add_prop_string("compatible", "allwinner,sun20i-d1-emac");
        builder.add_prop_reg64(0x0450_0000, 0x1000);
        builder.add_prop_u32("interrupts", 62); // EMAC interrupt
        builder.add_prop_u32("interrupt-parent", 100);
        builder.add_prop_string("status", "okay");
        builder.end_node();
    }
    
    // D1 I2C2 with GT911 touchscreen @ 0x0250_2000
    if d1_config.has_touch {
        builder.begin_node("i2c@2502000");
        builder.add_prop_string("compatible", "allwinner,sun20i-d1-i2c");
        builder.add_prop_reg64(0x0250_2000, 0x400);
        builder.add_prop_u32("#address-cells", 1);
        builder.add_prop_u32("#size-cells", 0);
        builder.add_prop_u32("interrupts", 25); // I2C2 interrupt
        builder.add_prop_u32("interrupt-parent", 100);
        builder.add_prop_string("status", "okay");
        
        // GT911 touchscreen @ I2C address 0x14
        builder.begin_node("touchscreen@14");
        builder.add_prop_string("compatible", "goodix,gt911");
        builder.add_prop_u32("reg", 0x14);
        builder.add_prop_u32("interrupts", 35); // GPIO interrupt
        builder.add_prop_u32("interrupt-parent", 100);
        builder.add_prop_string("status", "okay");
        builder.end_node(); // touchscreen
        
        builder.end_node(); // i2c
    }
    
    builder.end_node(); // /soc
    builder.end_node(); // /
    
    builder.finish()
}

/// Write the DTB to DRAM at the standard location.
///
/// # Returns
/// The physical address where the DTB was written.
pub fn write_dtb_to_dram(dram: &Dram, dtb: &[u8]) -> u64 {
    let offset = DTB_ADDRESS - 0x8000_0000; // Offset from DRAM base
    for (i, byte) in dtb.iter().enumerate() {
        let _ = dram.store_8(offset + i as u64, *byte as u64);
    }
    DTB_ADDRESS
}

/// Simple DTB builder that constructs a valid FDT blob.
struct DtbBuilder {
    struct_block: Vec<u8>,
    strings_block: Vec<u8>,
    string_offsets: std::collections::HashMap<String, u32>,
}

impl DtbBuilder {
    fn new() -> Self {
        Self {
            struct_block: Vec::new(),
            strings_block: Vec::new(),
            string_offsets: std::collections::HashMap::new(),
        }
    }
    
    fn begin_node(&mut self, name: &str) {
        self.write_u32(FDT_BEGIN_NODE);
        self.write_string(name);
        self.align4();
    }
    
    fn end_node(&mut self) {
        self.write_u32(FDT_END_NODE);
    }
    
    fn add_prop_string(&mut self, name: &str, value: &str) {
        let string_offset = self.get_string_offset(name);
        let value_bytes = value.as_bytes();
        
        self.write_u32(FDT_PROP);
        self.write_u32((value_bytes.len() + 1) as u32); // +1 for null terminator
        self.write_u32(string_offset);
        self.struct_block.extend_from_slice(value_bytes);
        self.struct_block.push(0); // null terminator
        self.align4();
    }
    
    fn add_prop_u32(&mut self, name: &str, value: u32) {
        let string_offset = self.get_string_offset(name);
        
        self.write_u32(FDT_PROP);
        self.write_u32(4);
        self.write_u32(string_offset);
        self.write_u32(value);
    }
    
    fn add_prop_u32_array(&mut self, name: &str, values: &[u32]) {
        let string_offset = self.get_string_offset(name);
        
        self.write_u32(FDT_PROP);
        self.write_u32((values.len() * 4) as u32);
        self.write_u32(string_offset);
        for value in values {
            self.write_u32(*value);
        }
    }
    
    fn add_prop_reg64(&mut self, address: u64, size: u64) {
        let string_offset = self.get_string_offset("reg");
        
        self.write_u32(FDT_PROP);
        self.write_u32(16); // 2 cells address + 2 cells size
        self.write_u32(string_offset);
        self.write_u32((address >> 32) as u32);
        self.write_u32(address as u32);
        self.write_u32((size >> 32) as u32);
        self.write_u32(size as u32);
    }
    
    fn add_prop_empty(&mut self, name: &str) {
        let string_offset = self.get_string_offset(name);
        
        self.write_u32(FDT_PROP);
        self.write_u32(0);
        self.write_u32(string_offset);
    }
    
    fn get_string_offset(&mut self, name: &str) -> u32 {
        if let Some(&offset) = self.string_offsets.get(name) {
            return offset;
        }
        
        let offset = self.strings_block.len() as u32;
        self.strings_block.extend_from_slice(name.as_bytes());
        self.strings_block.push(0); // null terminator
        self.string_offsets.insert(name.to_string(), offset);
        offset
    }
    
    fn write_u32(&mut self, value: u32) {
        self.struct_block.extend_from_slice(&value.to_be_bytes());
    }
    
    fn write_string(&mut self, s: &str) {
        self.struct_block.extend_from_slice(s.as_bytes());
        self.struct_block.push(0);
    }
    
    fn align4(&mut self) {
        while self.struct_block.len() % 4 != 0 {
            self.struct_block.push(0);
        }
    }
    
    fn finish(mut self) -> Vec<u8> {
        self.write_u32(FDT_END);
        
        // Calculate sizes and offsets
        let header_size = 40u32; // FDT header is 40 bytes
        let struct_size = self.struct_block.len() as u32;
        let strings_size = self.strings_block.len() as u32;
        
        // Memory reservation block (empty, 16 bytes of zeros)
        let mem_rsvmap_off = header_size;
        let struct_off = mem_rsvmap_off + 16;
        let strings_off = struct_off + struct_size;
        let total_size = strings_off + strings_size;
        
        // Build the complete DTB
        let mut dtb = Vec::with_capacity(total_size as usize);
        
        // Header
        dtb.extend_from_slice(&FDT_MAGIC.to_be_bytes());
        dtb.extend_from_slice(&total_size.to_be_bytes());
        dtb.extend_from_slice(&struct_off.to_be_bytes());
        dtb.extend_from_slice(&strings_off.to_be_bytes());
        dtb.extend_from_slice(&mem_rsvmap_off.to_be_bytes());
        dtb.extend_from_slice(&FDT_VERSION.to_be_bytes());
        dtb.extend_from_slice(&FDT_LAST_COMP_VERSION.to_be_bytes());
        dtb.extend_from_slice(&0u32.to_be_bytes()); // boot_cpuid_phys
        dtb.extend_from_slice(&strings_size.to_be_bytes());
        dtb.extend_from_slice(&struct_size.to_be_bytes());
        
        // Memory reservation block (empty)
        dtb.extend_from_slice(&[0u8; 16]);
        
        // Structure block
        dtb.extend_from_slice(&self.struct_block);
        
        // Strings block
        dtb.extend_from_slice(&self.strings_block);
        
        dtb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_dtb() {
        let config = D1DeviceConfig {
            has_display: true,
            has_mmc: true,
            has_emac: true,
        };
        let dtb = generate_dtb(2, 512 * 1024 * 1024, &config);
        
        // Verify magic number
        assert_eq!(dtb[0..4], FDT_MAGIC.to_be_bytes());
        
        // Verify it's not empty
        assert!(dtb.len() > 100);
        
        // Verify it's within size limits
        assert!(dtb.len() < DTB_MAX_SIZE);
    }
    
    #[test]
    fn test_dtb_structure() {
        let config = D1DeviceConfig::default();
        let dtb = generate_dtb(1, 256 * 1024 * 1024, &config);
        
        // DTB should start with magic number
        assert_eq!(dtb[0..4], FDT_MAGIC.to_be_bytes());
        
        // DTB should be at least header size (40 bytes) + mem_rsv (16) + some content
        assert!(dtb.len() > 60);
        
        // Verify version field in header (offset 0x14)
        let version = u32::from_be_bytes([dtb[20], dtb[21], dtb[22], dtb[23]]);
        assert_eq!(version, FDT_VERSION);
    }
}
