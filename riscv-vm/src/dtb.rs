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
use crate::machine::{AttachedDevices, Machine, ISA_STRING};

/// DTB location on the **virt** map (DRAM `0x8000_0000` + 32 MiB).
/// Prefer `Machine::memory_map().dtb_addr()` for new code.
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

/// Devices present when the DTB is generated (legacy name).
pub type D1DeviceConfig = AttachedDevices;

/// Generate a DTB for `Machine::Virt` (legacy wrapper).
///
/// New code should call [`generate_for`] so D1 does not inherit the virt map.
pub fn generate_dtb(
    num_harts: usize,
    memory_size: u64,
    d1_config: &D1DeviceConfig,
) -> Vec<u8> {
    generate_for(Machine::Virt, num_harts, memory_size, d1_config)
}

/// Generate a DTB whose `compatible`, map, ISA, and timebase match `machine`.
///
/// Only instantiated devices are listed. Call again after attach (GPU/disk/virtio).
pub fn generate_for(
    machine: Machine,
    num_harts: usize,
    memory_size: u64,
    attached: &AttachedDevices,
) -> Vec<u8> {
    let map = machine.memory_map();
    let mut builder = DtbBuilder::new();

    builder.begin_node("");
    builder.add_prop_u32("#address-cells", 2);
    builder.add_prop_u32("#size-cells", 2);
    builder.add_prop_string_list("compatible", map.compatible);
    builder.add_prop_string("model", map.model);

    builder.begin_node("chosen");
    builder.add_prop_string("bootargs", "earlycon=sbi console=ttyS0");
    builder.add_prop_string(
        "stdout-path",
        &format!("/soc/serial@{:x}", map.uart.base),
    );
    // Optional `/chosen` fallback the kernel also parses. Primary advertisement
    // is the reserved-memory node below; omit both when HDL is disabled.
    if advertise_hdl(machine, attached) {
        builder.add_prop_reg64_named(
            "havy,hdl-mailbox",
            crate::hdl::HDL_ADDR_VIRT,
            crate::hdl::HDL_REGION_SIZE as u64,
        );
        builder.add_prop_u32("havy,hdl-abi-major", crate::hdl::HDL_ABI_MAJOR as u32);
    }
    builder.end_node();

    builder.begin_node("cpus");
    builder.add_prop_u32("#address-cells", 1);
    builder.add_prop_u32("#size-cells", 0);
    builder.add_prop_u32("timebase-frequency", map.timebase_hz);

    for hart in 0..num_harts {
        builder.begin_node(&format!("cpu@{}", hart));
        builder.add_prop_string("device_type", "cpu");
        builder.add_prop_u32("reg", hart as u32);
        builder.add_prop_string("status", "okay");
        builder.add_prop_string("compatible", "riscv");
        builder.add_prop_string("riscv,isa", ISA_STRING);
        builder.add_prop_string("mmu-type", "riscv,sv39");

        builder.begin_node("interrupt-controller");
        builder.add_prop_u32("#interrupt-cells", 1);
        builder.add_prop_empty("interrupt-controller");
        builder.add_prop_string("compatible", "riscv,cpu-intc");
        builder.add_prop_u32("phandle", (hart + 1) as u32);
        builder.end_node();

        builder.end_node();
    }
    builder.end_node();

    builder.begin_node(&format!("memory@{:x}", map.dram_base));
    builder.add_prop_string("device_type", "memory");
    builder.add_prop_reg64(map.dram_base, memory_size);
    builder.end_node();

    builder.begin_node("reserved-memory");
    builder.add_prop_u32("#address-cells", 2);
    builder.add_prop_u32("#size-cells", 2);
    builder.add_prop_empty("ranges");
    let fb_meta = map.dram_base + 0x00FF_F000;
    builder.begin_node(&format!("fb-doorbell@{:x}", fb_meta));
    builder.add_prop_string("compatible", "havy,fb-doorbell");
    builder.add_prop_reg64(fb_meta, 0x1000);
    builder.end_node();
    let fb_base = map.dram_base + 0x0100_0000;
    let fb_bytes: u64 = match machine {
        Machine::Virt => 4096 * 768,
        Machine::D1 => 2048 * 480,
    };
    builder.begin_node(&format!("framebuffer@{:x}", fb_base));
    builder.add_prop_string("compatible", "simple-framebuffer");
    builder.add_prop_reg64(fb_base, fb_bytes);
    builder.end_node();
    if advertise_hdl(machine, attached) {
        let hdl_base = crate::hdl::HDL_ADDR_VIRT;
        builder.begin_node(&format!("hdl-mailbox@{:x}", hdl_base));
        builder.add_prop_string("compatible", "havy,hdl-mailbox");
        builder.add_prop_reg64(hdl_base, crate::hdl::HDL_REGION_SIZE as u64);
        builder.add_prop_u32("havy,abi-major", crate::hdl::HDL_ABI_MAJOR as u32);
        builder.add_prop_u32("havy,abi-minor", crate::hdl::HDL_ABI_MINOR as u32);
        builder.add_prop_empty("no-map");
        builder.end_node();
    }
    builder.end_node();

    builder.begin_node("soc");
    builder.add_prop_u32("#address-cells", 2);
    builder.add_prop_u32("#size-cells", 2);
    builder.add_prop_string("compatible", "simple-bus");
    builder.add_prop_empty("ranges");

    if let Some(clint) = map.clint {
        builder.begin_node(&format!("clint@{:x}", clint.base));
        builder.add_prop_string("compatible", "riscv,clint0");
        builder.add_prop_reg64(clint.base, clint.size);
        let mut clint_ints = Vec::new();
        for hart in 0..num_harts {
            clint_ints.push((hart + 1) as u32);
            clint_ints.push(3);
            clint_ints.push((hart + 1) as u32);
            clint_ints.push(7);
        }
        builder.add_prop_u32_array("interrupts-extended", &clint_ints);
        builder.end_node();
    }

    builder.begin_node(&format!("interrupt-controller@{:x}", map.plic.base));
    builder.add_prop_string("compatible", map.plic.compatible);
    builder.add_prop_u32("#interrupt-cells", 1);
    builder.add_prop_empty("interrupt-controller");
    let plic_dtb_size = match machine {
        Machine::Virt => crate::machine::virt::PLIC_DTB_SIZE,
        Machine::D1 => map.plic.size,
    };
    builder.add_prop_reg64(map.plic.base, plic_dtb_size);
    builder.add_prop_u32("riscv,ndev", map.plic.ndev);
    builder.add_prop_u32("phandle", 100);
    // The VM's emulated devices and PLIC are owned by hart 0. Secondary
    // harts use guest IPIs and the kernel I/O router, so advertise only the
    // BSP's supervisor/machine external-interrupt contexts.
    let plic_ints = vec![1, 9, 1, 11];
    builder.add_prop_u32_array("interrupts-extended", &plic_ints);
    builder.end_node();

    builder.begin_node(&format!("serial@{:x}", map.uart.base));
    builder.add_prop_string("compatible", map.uart.compatible);
    builder.add_prop_reg64(map.uart.base, map.uart.size);
    builder.add_prop_u32("clock-frequency", 3686400);
    builder.add_prop_u32("interrupts", map.uart.irq);
    builder.add_prop_u32("interrupt-parent", 100);
    builder.end_node();

    if let Some(virtio) = map.virtio {
        let n = attached.virtio_count.min(virtio.max_slots);
        for i in 0..n as u64 {
            let addr = virtio.base + i * virtio.stride;
            builder.begin_node(&format!("virtio@{:x}", addr));
            builder.add_prop_string("compatible", "virtio,mmio");
            builder.add_prop_reg64(addr, virtio.stride);
            builder.add_prop_u32("interrupts", (1 + i) as u32);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.end_node();
        }
    }

    // D1 peripherals belong on Machine::D1 only (never on a virt-compatible DTB).
    if machine == Machine::D1 {
        if attached.has_display {
            builder.begin_node("display-engine@5100000");
            builder.add_prop_string("compatible", "allwinner,sun20i-d1-de2");
            builder.add_prop_reg64(0x0510_0000, 0x10000);
            builder.add_prop_u32("interrupts", 42);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.add_prop_string("status", "okay");
            builder.end_node();

            builder.begin_node("lcd-controller@5461000");
            builder.add_prop_string("compatible", "allwinner,sun20i-d1-tcon-lcd");
            builder.add_prop_reg64(0x0546_1000, 0x1000);
            builder.add_prop_u32("interrupts", 106);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.add_prop_string("status", "okay");
            builder.end_node();
        }

        if attached.has_mmc {
            builder.begin_node("mmc@4020000");
            builder.add_prop_string("compatible", "allwinner,sun20i-d1-mmc");
            builder.add_prop_reg64(0x0402_0000, 0x1000);
            builder.add_prop_u32("interrupts", 56);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.add_prop_string("status", "okay");
            builder.end_node();
        }

        if attached.has_emac {
            builder.begin_node("ethernet@4500000");
            builder.add_prop_string("compatible", "allwinner,sun20i-d1-emac");
            builder.add_prop_reg64(0x0450_0000, 0x1000);
            builder.add_prop_u32("interrupts", 62);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.add_prop_string("status", "okay");
            builder.end_node();
        }

        if attached.has_touch {
            builder.begin_node("i2c@2502000");
            builder.add_prop_string("compatible", "allwinner,sun20i-d1-i2c");
            builder.add_prop_reg64(0x0250_2000, 0x400);
            builder.add_prop_u32("#address-cells", 1);
            builder.add_prop_u32("#size-cells", 0);
            builder.add_prop_u32("interrupts", 25);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.add_prop_string("status", "okay");

            builder.begin_node("touchscreen@14");
            builder.add_prop_string("compatible", "goodix,gt911");
            builder.add_prop_u32("reg", 0x14);
            builder.add_prop_u32("interrupts", 35);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.add_prop_string("status", "okay");
            builder.end_node();

            builder.end_node();
        }

        if attached.has_audio {
            builder.begin_node("codec@2030000");
            builder.add_prop_string("compatible", "allwinner,sun20i-d1-codec");
            builder.add_prop_reg64(0x0203_0000, 0x1000);
            builder.add_prop_u32("interrupts", 32);
            builder.add_prop_u32("interrupt-parent", 100);
            builder.add_prop_string("status", "okay");
            builder.end_node();
        }
    }

    builder.end_node();
    builder.end_node();

    builder.finish()
}

/// Virt DTB advertises the mailbox only when the host kill-switch is on.
/// D1 never publishes this node (silicon has no host GPU mailbox).
fn advertise_hdl(machine: Machine, attached: &AttachedDevices) -> bool {
    machine == Machine::Virt && attached.hdl_mailbox
}

/// Write the DTB at this machine's DTB physical address (`dram.base + dtb_offset`).
pub fn write_dtb_to_dram(dram: &Dram, dtb: &[u8]) -> u64 {
    write_dtb_to_dram_at(dram, dtb, dtb_addr_for_dram(dram))
}

/// DTB PA for a DRAM whose `base` matches a known [`Machine`].
pub fn dtb_addr_for_dram(dram: &Dram) -> u64 {
    Machine::from_dram_base(dram.base)
        .unwrap_or(Machine::Virt)
        .memory_map()
        .dtb_addr()
}

/// Write `dtb` at an explicit guest physical address.
pub fn write_dtb_to_dram_at(dram: &Dram, dtb: &[u8], dtb_addr: u64) -> u64 {
    assert!(dtb.len() <= DTB_MAX_SIZE, "DTB exceeds {} bytes", DTB_MAX_SIZE);
    let offset = dtb_addr.checked_sub(dram.base).expect("DTB address below DRAM");
    dram.load(dtb, offset).expect("DTB write outside guest DRAM");
    let magic = dram
        .read_range(offset as usize, FDT_MAGIC.to_be_bytes().len())
        .expect("DTB readback outside guest DRAM");
    assert_eq!(magic.as_slice(), FDT_MAGIC.to_be_bytes());
    dtb_addr
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

    fn add_prop_string_list(&mut self, name: &str, values: &[&str]) {
        let string_offset = self.get_string_offset(name);
        let mut blob = Vec::new();
        for value in values {
            blob.extend_from_slice(value.as_bytes());
            blob.push(0);
        }
        self.write_u32(FDT_PROP);
        self.write_u32(blob.len() as u32);
        self.write_u32(string_offset);
        self.struct_block.extend_from_slice(&blob);
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
        self.add_prop_reg64_named("reg", address, size);
    }

    fn add_prop_reg64_named(&mut self, name: &str, address: u64, size: u64) {
        let string_offset = self.get_string_offset(name);

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
        let config = AttachedDevices {
            has_display: true,
            has_mmc: true,
            has_emac: true,
            has_touch: true,
            has_audio: true,
            virtio_count: 2,
            hdl_mailbox: true,
        };
        let dtb = generate_for(Machine::Virt, 2, 512 * 1024 * 1024, &config);
        assert_eq!(dtb[0..4], FDT_MAGIC.to_be_bytes());
        assert!(dtb.len() > 100);
        assert!(dtb.len() < DTB_MAX_SIZE);
        let text = String::from_utf8_lossy(&dtb);
        assert!(text.contains("riscv-virtio,qemu"));
        assert!(!text.contains("allwinner,sun20i-d1"));
        assert!(text.contains(ISA_STRING));
        assert!(dtb.windows(6).any(|w| w == b"cpu@0\0"));
        assert!(dtb.windows(6).any(|w| w == b"cpu@1\0"));
        assert!(!dtb.windows(6).any(|w| w == b"cpu@2\0"));
    }

    #[test]
    fn virt_dtb_contains_every_requested_hart() {
        for num_harts in [1usize, 2, 4, 8] {
            let dtb = generate_for(
                Machine::Virt,
                num_harts,
                512 * 1024 * 1024,
                &AttachedDevices::default(),
            );
            for hart in 0..num_harts {
                let node = format!("cpu@{}\0", hart);
                assert!(
                    dtb.windows(node.len()).any(|window| window == node.as_bytes()),
                    "missing {node:?} from {num_harts}-hart DTB",
                );
            }
            let extra = format!("cpu@{}\0", num_harts);
            assert!(
                !dtb.windows(extra.len()).any(|window| window == extra.as_bytes()),
                "unexpected {extra:?} in {num_harts}-hart DTB",
            );
        }
    }

    #[test]
    fn d1_dtb_uses_d1_map() {
        let dtb = generate_for(
            Machine::D1,
            1,
            512 * 1024 * 1024,
            &AttachedDevices {
                has_mmc: true,
                has_emac: true,
                ..Default::default()
            },
        );
        let text = String::from_utf8_lossy(&dtb);
        assert!(text.contains("allwinner,sun20i-d1"));
        assert!(text.contains("memory@40000000"));
        assert!(text.contains("serial@2500000"));
        assert!(!text.contains("virtio,mmio"));
        assert!(!text.contains("riscv,clint0"));
        assert!(!text.contains("havy,hdl-mailbox"));
        assert!(!text.contains("hdl-mailbox@81400000"));
    }

    #[test]
    fn virt_dtb_advertises_hdl_mailbox_when_enabled() {
        let dtb = generate_for(
            Machine::Virt,
            1,
            512 * 1024 * 1024,
            &AttachedDevices {
                hdl_mailbox: true,
                ..Default::default()
            },
        );
        let text = String::from_utf8_lossy(&dtb);
        assert!(text.contains("hdl-mailbox@81400000"));
        assert!(text.contains("havy,hdl-mailbox"));
        assert!(text.contains("havy,abi-major"));
        assert!(text.contains("havy,abi-minor"));
        assert!(text.contains("no-map"));
        assert!(text.contains("havy,hdl-abi-major"));
        // reg = <0x0 0x81400000 0x0 0x21000> as four big-endian cells.
        let reg = [
            0u32.to_be_bytes(),
            0x8140_0000u32.to_be_bytes(),
            0u32.to_be_bytes(),
            0x0002_1000u32.to_be_bytes(),
        ]
        .concat();
        assert!(
            dtb.windows(reg.len()).any(|w| w == reg.as_slice()),
            "missing HDL mailbox reg cells"
        );
    }

    #[test]
    fn virt_dtb_omits_hdl_mailbox_when_disabled() {
        let dtb = generate_for(
            Machine::Virt,
            1,
            512 * 1024 * 1024,
            &AttachedDevices {
                hdl_mailbox: false,
                ..Default::default()
            },
        );
        let text = String::from_utf8_lossy(&dtb);
        assert!(!text.contains("hdl-mailbox@"));
        assert!(!text.contains("havy,hdl-mailbox"));
        assert!(!text.contains("havy,hdl-abi-major"));
        assert!(!text.contains("havy,abi-major"));
    }

    #[test]
    fn d1_dtb_never_advertises_hdl_even_if_flag_set() {
        let dtb = generate_for(
            Machine::D1,
            1,
            512 * 1024 * 1024,
            &AttachedDevices {
                hdl_mailbox: true,
                ..Default::default()
            },
        );
        let text = String::from_utf8_lossy(&dtb);
        assert!(!text.contains("havy,hdl-mailbox"));
        assert!(!text.contains("hdl-mailbox@"));
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
