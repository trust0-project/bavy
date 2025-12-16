use clap::Parser;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use riscv_vm::sdboot;
use riscv_vm::vm::native::NativeVm;

#[derive(Parser, Debug)]
#[command(name = "riscv-vm")]
#[command(about = "RISCV emulator with SMP support")]
#[command(version)]
struct Args {
    /// Path to SD card image (contains kernel + filesystem)
    #[arg(short, long)]
    sdcard: PathBuf,

    /// Number of harts (CPUs), 0 for auto-detect
    #[arg(short = 'n', long, default_value = "0")]
    harts: usize,

    /// WebTransport relay URL for networking (e.g., https://127.0.0.1:4433)
    #[arg(long)]
    net_webtransport: Option<String>,

    /// Certificate hash for WebTransport (for self-signed certs)
    #[arg(long)]
    cert_hash: Option<String>,

    /// Enable debug output
    #[arg(long)]
    debug: bool,
}

/// Write to stdout with \r\n line endings (for raw terminal mode)
fn uart_print(s: &str) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for byte in s.bytes() {
        if byte == b'\n' {
            let _ = out.write_all(b"\r\n");
        } else {
            let _ = out.write_all(&[byte]);
        }
    }
    let _ = out.flush();
}

/// Write formatted output to stdout with \r\n, adding a newline at the end
macro_rules! uart_println {
    () => { uart_print("\n") };
    ($($arg:tt)*) => {{
        uart_print(&format!($($arg)*));
        uart_print("\n");
    }};
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    if args.debug {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Load SD card image
    let sdcard_data = fs::read(&args.sdcard)
        .map_err(|e| format!("Failed to read SD card image '{}': {}", args.sdcard.display(), e))?;

    // Parse SD card: find kernel on boot partition
    let boot_info = sdboot::parse_sdcard(&sdcard_data)
        .map_err(|e| format!("Failed to parse SD card: {}", e))?;

    // Determine hart count
    let num_harts = if args.harts == 0 {
        // Auto-detect: use all available CPUs since idle harts sleep via WFI
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        args.harts
    }
    .max(1);

    // Print banner
    uart_println!();
    uart_println!("╔══════════════════════════════════════════════════════════════╗");
    uart_println!("║  RISCV-VM with OpenSBI                                       ║");
    uart_println!("╠══════════════════════════════════════════════════════════════╣");
    uart_println!(
        "║  SD Card: {:52} ║",
        args.sdcard
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    );
    uart_println!("║  Kernel:  {} bytes @ {:#x}{:>23} ║", 
        boot_info.kernel_data.len(),
        boot_info.kernel_load_addr,
        ""
    );
    uart_println!("║  Harts:   {:52} ║", num_harts);
    if let Some(relay) = &args.net_webtransport {
        uart_println!("║  Network: {:52} ║", relay);
    }
    uart_println!("╚══════════════════════════════════════════════════════════════╝");
    uart_println!();

    // Create VM with kernel from SD card
    let mut vm = NativeVm::new(&boot_info.kernel_data, num_harts)?;

    // Load entire SD card as block device (for filesystem partition)
    vm.load_disk(sdcard_data);
    uart_println!("[VM] SD card mounted (fs partition at sector {})", boot_info.fs_partition_start);

    // Connect to WebTransport relay if specified
    if let Some(relay_url) = &args.net_webtransport {
        vm.connect_webtransport(relay_url, args.cert_hash.clone());
    }

    // Run VM
    vm.run();

    // Report exit status
    let halt_code = vm.shared.halt_code();
    if halt_code == 0x5555 {
        uart_println!();
        uart_println!("[VM] Clean shutdown (PASS)");
        Ok(())
    } else {
        uart_println!();
        uart_println!("[VM] Shutdown with code: {:#x}", halt_code);
        Ok(())
    }
}

