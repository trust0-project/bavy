//! SBI Debug Console Extension (EID 0x4442434E "DBCN")
//!
//! Provides debug console I/O functionality per SBI v2.0 spec.

use super::SbiRet;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::devices::uart::UART_BASE;
use crate::engine::decoder::Register;

// ============================================================================
// Function IDs
// ============================================================================

/// Console Write (FID 0)
const FID_CONSOLE_WRITE: u64 = 0;
/// Console Read (FID 1)
const FID_CONSOLE_READ: u64 = 1;
/// Console Write Byte (FID 2)
const FID_CONSOLE_WRITE_BYTE: u64 = 2;

// ============================================================================
// Handler
// ============================================================================

/// Handle Debug Console Extension calls.
pub fn handle(cpu: &Cpu, bus: &dyn Bus, fid: u64) -> SbiRet {
    match fid {
        FID_CONSOLE_WRITE => console_write(cpu, bus),
        FID_CONSOLE_READ => console_read(cpu, bus),
        FID_CONSOLE_WRITE_BYTE => console_write_byte(cpu, bus),
        _ => SbiRet::not_supported(),
    }
}

/// Console Write (FID 0)
///
/// Writes bytes from memory to the console.
///
/// # Arguments
/// * `a0` - Number of bytes to write
/// * `a1` - Base address (low 32 bits for RV32, full address for RV64)
/// * `a2` - Base address (high 32 bits for RV32, ignored for RV64)
///
/// # Returns
/// * Number of bytes successfully written in value
fn console_write(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let num_bytes = cpu.read_reg(Register::X10); // a0
    let base_addr_lo = cpu.read_reg(Register::X11); // a1
    let _base_addr_hi = cpu.read_reg(Register::X12); // a2 (ignored on RV64)

    // On RV64, the full address is in a1
    let base_addr = base_addr_lo;

    let mut bytes_written = 0u64;

    for i in 0..num_bytes {
        // Read byte from memory
        let byte = match bus.read8(base_addr.wrapping_add(i)) {
            Ok(b) => b,
            Err(_) => break,
        };

        // Write to UART THR
        if let Err(_) = bus.write8(UART_BASE, byte) {
            break;
        }

        bytes_written += 1;
    }

    SbiRet::success(bytes_written as i64)
}

/// Console Read (FID 1)
///
/// Reads bytes from the console into memory.
///
/// # Arguments
/// * `a0` - Maximum number of bytes to read
/// * `a1` - Base address (low 32 bits for RV32, full address for RV64)
/// * `a2` - Base address (high 32 bits for RV32, ignored for RV64)
///
/// # Returns
/// * Number of bytes successfully read in value
fn console_read(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let num_bytes = cpu.read_reg(Register::X10); // a0
    let base_addr_lo = cpu.read_reg(Register::X11); // a1
    let _base_addr_hi = cpu.read_reg(Register::X12); // a2 (ignored on RV64)

    // On RV64, the full address is in a1
    let base_addr = base_addr_lo;

    let mut bytes_read = 0u64;

    for i in 0..num_bytes {
        // Check LSR for data ready
        let lsr = match bus.read8(UART_BASE + 5) {
            Ok(v) => v,
            Err(_) => break,
        };

        if (lsr & 1) == 0 {
            // No more data available
            break;
        }

        // Read from UART RBR
        let byte = match bus.read8(UART_BASE) {
            Ok(b) => b,
            Err(_) => break,
        };

        // Write to memory
        if let Err(_) = bus.write8(base_addr.wrapping_add(i), byte) {
            break;
        }

        bytes_read += 1;
    }

    SbiRet::success(bytes_read as i64)
}

/// Console Write Byte (FID 2)
///
/// Writes a single byte to the console.
///
/// # Arguments
/// * `a0` - Byte to write (in low 8 bits)
///
/// # Returns
/// * SBI_SUCCESS on success
fn console_write_byte(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let byte = (cpu.read_reg(Register::X10) & 0xFF) as u8; // a0

    // Write to UART THR
    if let Err(_) = bus.write8(UART_BASE, byte) {
        return SbiRet::failed();
    }

    SbiRet::ok()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fid_values() {
        assert_eq!(FID_CONSOLE_WRITE, 0);
        assert_eq!(FID_CONSOLE_READ, 1);
        assert_eq!(FID_CONSOLE_WRITE_BYTE, 2);
    }
}
