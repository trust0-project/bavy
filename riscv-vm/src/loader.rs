//! Binary and ELF loading utilities.

use crate::bus::SystemBus;
use goblin::elf::{Elf, program_header::PT_LOAD};

/// Load an ELF kernel into DRAM (Native version).
///
/// Takes a shared reference to the bus since SystemBus uses interior
/// mutability for DRAM access via `UnsafeCell`.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_elf_into_dram(buffer: &[u8], bus: &SystemBus) -> Result<u64, String> {
    let elf = Elf::parse(buffer).map_err(|e| format!("ELF parse error: {}", e))?;
    let base = bus.dram.base;
    let dram_size = bus.dram.size();
    let dram_end = base + dram_size as u64;

    for ph in &elf.program_headers {
        if ph.p_type != PT_LOAD || ph.p_memsz == 0 {
            continue;
        }

        let file_size = ph.p_filesz as usize;
        let mem_size = ph.p_memsz as usize;
        let file_offset = ph.p_offset as usize;

        if file_offset + file_size > buffer.len() {
            return Err("Segment exceeds file bounds".to_string());
        }

        let target_addr = if ph.p_paddr != 0 {
            ph.p_paddr
        } else {
            ph.p_vaddr
        };

        if target_addr < base || target_addr + mem_size as u64 > dram_end {
            return Err(format!("Segment 0x{:x} out of DRAM range", target_addr));
        }

        let dram_offset = target_addr - base;

        if file_size > 0 {
            bus.dram
                .load(&buffer[file_offset..file_offset + file_size], dram_offset)
                .map_err(|e| format!("Failed to load segment: {:?}", e))?;
        }

        if mem_size > file_size {
            bus.dram
                .zero_range((dram_offset as usize) + file_size, mem_size - file_size)
                .map_err(|e| format!("Failed to zero BSS: {:?}", e))?;
        }
    }

    log::debug!(
        "ELF loaded: entry=0x{:x}, segments={}",
        elf.entry,
        elf.program_headers.len()
    );

    Ok(elf.entry)
}

/// Load an ELF kernel into DRAM (WASM-compatible version).
///
/// This version is separate because `goblin` usage might vary slightly
/// or we might want different error handling in WASM.
#[cfg(target_arch = "wasm32")]
pub fn load_elf_wasm(buffer: &[u8], bus: &SystemBus) -> Result<u64, String> {
    let elf = Elf::parse(buffer).map_err(|e| format!("ELF parse error: {}", e))?;
    let base = bus.dram_base();
    let dram_end = base + bus.dram_size() as u64;

    for ph in &elf.program_headers {
        if ph.p_type != PT_LOAD || ph.p_memsz == 0 {
            continue;
        }

        let file_size = ph.p_filesz as usize;
        let mem_size = ph.p_memsz as usize;
        let file_offset = ph.p_offset as usize;
        if file_offset + file_size > buffer.len() {
            return Err(format!(
                "ELF segment exceeds file bounds (offset 0x{:x})",
                file_offset
            ));
        }

        let target_addr = if ph.p_paddr != 0 {
            ph.p_paddr
        } else {
            ph.p_vaddr
        };
        if target_addr < base {
            return Err(format!(
                "Segment start 0x{:x} lies below DRAM base 0x{:x}",
                target_addr, base
            ));
        }
        let seg_end = target_addr
            .checked_add(mem_size as u64)
            .ok_or_else(|| "Segment end overflow".to_string())?;
        if seg_end > dram_end {
            return Err(format!(
                "Segment 0x{:x}-0x{:x} exceeds DRAM (end 0x{:x})",
                target_addr, seg_end, dram_end
            ));
        }

        let dram_offset = (target_addr - base) as u64;
        if file_size > 0 {
            let end = file_offset + file_size;
            bus.dram
                .load(&buffer[file_offset..end], dram_offset)
                .map_err(|e| format!("Failed to load segment: {}", e))?;
        }
        if mem_size > file_size {
            let zero_start = dram_offset as usize + file_size;
            bus.dram
                .zero_range(zero_start, mem_size - file_size)
                .map_err(|e| format!("Failed to zero bss: {}", e))?;
        }
    }

    Ok(elf.entry)
}








