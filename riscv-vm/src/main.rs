use clap::Parser;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use riscv_vm::Machine;
use riscv_vm::sdboot;
use riscv_vm::vm::native::NativeVm;

fn parse_hdl_flag(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(format!(
            "invalid --hdl / HAVY_HDL value {other:?}; expected 0 or 1"
        )),
    }
}

#[derive(Parser, Debug)]
#[command(name = "riscv-vm")]
#[command(about = "RISCV emulator with SMP support")]
#[command(version)]
struct Args {
    /// Path or URL to SD card image (contains kernel + filesystem)
    /// Supports local files or http:// / https:// URLs
    #[arg(short, long, required_unless_present = "bench")]
    sdcard: Option<String>,

    /// Run a synthetic benchmark workload instead of booting an SD card.
    /// Workloads: nop, prime, memcpy, spinlock, ecall, all
    #[arg(long)]
    bench: Option<String>,

    /// Benchmark duration per workload, in seconds
    #[arg(long, default_value = "3.0")]
    bench_seconds: f64,

    /// Number of harts (CPUs), 0 for auto-detect
    #[arg(short = 'n', long, default_value = "0")]
    harts: usize,

    /// Guest board: virt (QEMU-virt, 10 MHz timebase) or d1 (Allwinner, 24 MHz)
    #[arg(long, default_value = "virt")]
    machine: String,

    /// Advertise the HDL mailbox in the virt DTB (`1`/`true`, default).
    /// `--hdl=0` / `HAVY_HDL=0` omits the node (guest never reads env vars).
    #[arg(
        long,
        env = "HAVY_HDL",
        value_name = "0|1",
        default_value = "1",
        num_args = 0..=1,
        default_missing_value = "1",
        value_parser = parse_hdl_flag
    )]
    hdl: bool,

    /// WebTransport relay URL for networking (e.g., https://127.0.0.1:4433)
    #[arg(long)]
    net_webtransport: Option<String>,

    /// Certificate hash for WebTransport (for self-signed certs)
    #[arg(long)]
    cert_hash: Option<String>,

    /// Enable GPU display (opens a window)
    #[arg(long)]
    enable_gpu: bool,

    /// Window scale factor (1, 2, or 4) - only with --enable-gpu
    #[arg(long, default_value = "1")]
    scale: u8,

    /// Mount a host directory via VirtIO 9P (accessible at /mnt in guest)
    #[arg(long)]
    mount: Option<PathBuf>,

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

/// Load SD card data from a URL or local file path.
/// 
/// Supports:
/// - Local file paths (absolute or relative)
/// - HTTP/HTTPS URLs (downloads with progress display)
fn load_sdcard_data(source: &str, debug: bool) -> Result<Vec<u8>, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        // Download from URL
        if debug {
            eprintln!("[CLI] Downloading SD card from {}...", source);
        } else {
            eprintln!("Downloading SD card image...");
        }
        
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(30))
            .build();
        
        let response = agent.get(source).call()
            .map_err(|e| format!("Failed to download SD card from '{}': {}", source, e))?;
        
        // Check for success status
        let status = response.status();
        if status != 200 {
            return Err(format!("HTTP {} when downloading '{}'", status, source));
        }
        
        // Get content length if available for progress display
        let content_length = response.header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok());
        
        if let Some(len) = content_length {
            if debug {
                eprintln!("[CLI] Expected size: {} bytes", len);
            }
        }
        
        // Read the response body
        let mut data = if let Some(len) = content_length {
            Vec::with_capacity(len)
        } else {
            Vec::new()
        };
        
        response.into_reader().read_to_end(&mut data)
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        
        eprintln!("Downloaded {} bytes", data.len());
        
        Ok(data)
    } else {
        // Read from local file
        let path = Path::new(source);
        if !path.exists() {
            return Err(format!("SD card image not found at '{}'", source));
        }
        
        fs::read(path)
            .map_err(|e| format!("Failed to read SD card image '{}': {}", source, e))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Check if GUI is requested but feature not enabled
    #[cfg(not(feature = "gui"))]
    if args.enable_gpu {
        eprintln!("Error: --enable-gpu requires the 'gui' feature.");
        eprintln!("Rebuild with: cargo build --features gui");
        std::process::exit(1);
    }

    // Initialize logging
    if args.debug {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Benchmark mode: run synthetic workloads and exit.
    if let Some(bench_name) = &args.bench {
        let harts = if args.harts == 0 { 1 } else { args.harts };
        return run_bench(bench_name, args.bench_seconds, harts);
    }

    // Load SD card image (from URL or local file)
    let sdcard_source = args.sdcard.as_deref().expect("clap enforces sdcard");
    let sdcard_data = load_sdcard_data(sdcard_source, args.debug)?;

    // Parse SD card: find kernel on boot partition
    let boot_info = sdboot::parse_sdcard(&sdcard_data)
        .map_err(|e| format!("Failed to parse SD card: {}", e))?;

    let machine = Machine::parse(&args.machine).ok_or_else(|| {
        format!(
            "unknown machine {:?}; expected virt or d1",
            args.machine
        )
    })?;

    // Determine hart count.
    // `--harts 0` uses the machine default (virt: host CPUs, d1: 1).
    // D1 stays at 1 unless the user explicitly passes `--harts`.
    let host_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let num_harts = if args.harts == 0 {
        machine.default_harts_native(host_cpus)
    } else {
        args.harts
    }
    .max(1);

    // Print banner
    uart_println!();
    uart_println!("╔══════════════════════════════════════════════════════════════╗");
    if args.enable_gpu {
        uart_println!("║  RISCV-VM with OpenSBI (GUI)                                 ║");
    } else {
        uart_println!("║  RISCV-VM with OpenSBI                                       ║");
    }
    uart_println!("╠══════════════════════════════════════════════════════════════╣");
    // Extract display name from path or URL
    let sdcard_display = sdcard_source.rsplit('/').next()
        .unwrap_or(sdcard_source);
    let sdcard_display = if sdcard_display.len() > 52 {
        &sdcard_display[..52]
    } else {
        sdcard_display
    };
    uart_println!(
        "║  SD Card: {:52} ║",
        sdcard_display
    );
    uart_println!("║  Kernel:  {} bytes @ {:#x}{:>23} ║", 
        boot_info.kernel_data.len(),
        boot_info.kernel_load_addr,
        ""
    );
    uart_println!("║  Harts:   {:52} ║", num_harts);
    uart_println!(
        "║  Machine: {:52} ║",
        format!("{} (timebase {} Hz)", machine.as_str(), machine.timebase_hz())
    );
    uart_println!(
        "║  HDL:     {:52} ║",
        if args.hdl && machine == Machine::Virt {
            "advertised (DTB havy,hdl-mailbox)"
        } else {
            "omitted (kill-switch or d1)"
        }
    );
    if let Some(relay) = &args.net_webtransport {
        uart_println!("║  Network: {:52} ║", relay);
    }
    uart_println!("╚══════════════════════════════════════════════════════════════╝");
    uart_println!();

    // Create VM with kernel from SD card
    let hdl = args.hdl && machine == Machine::Virt;
    let mut vm = NativeVm::with_machine_hdl(&boot_info.kernel_data, num_harts, machine, hdl)?;

    // Load entire SD card as block device (for filesystem partition)
    vm.load_disk(sdcard_data);
    uart_println!("[VM] SD card mounted (fs partition at sector {})", boot_info.fs_partition_start);

    // Enable GPU if requested
    if args.enable_gpu {
        vm.enable_gpu(1024, 768);
    }

    // Enable host directory mounting via 9P if specified
    if let Some(mount_path) = &args.mount {
        let path_str = mount_path.to_string_lossy();
        vm.enable_9p(&path_str, None);
    }

    // Connect to WebTransport relay if specified
    if let Some(relay_url) = &args.net_webtransport {
        vm.connect_webtransport(relay_url, args.cert_hash.clone());
    }

    // Run VM - with or without GUI
    #[cfg(feature = "gui")]
    if args.enable_gpu {
        riscv_vm::hdl_gui::run(vm, args.scale)?;
    } else {
        run_headless(vm);
    }

    #[cfg(not(feature = "gui"))]
    run_headless(vm);

    Ok(())
}

/// Run synthetic benchmark workloads and print MIPS results.
fn run_bench(name: &str, seconds: f64, harts: usize) -> Result<(), Box<dyn std::error::Error>> {
    use riscv_vm::bench;

    let workloads: Vec<&str> = if name == "all" {
        bench::WORKLOADS.to_vec()
    } else {
        vec![name]
    };

    println!("riscv-vm benchmark | harts={harts} | {seconds:.1}s per workload");
    println!("{:<10} {:>8} {:>16} {:>10}", "workload", "harts", "instructions", "MIPS");
    println!("{}", "-".repeat(48));

    for workload in workloads {
        // Spinlock is only meaningful with >= 2 harts; bump automatically
        // when running the full suite with a single hart.
        let effective_harts = if workload == "spinlock" && harts == 1 && name == "all" {
            2
        } else {
            harts
        };
        let result = bench::run_native(workload, seconds, effective_harts)
            .map_err(|e| format!("bench '{workload}' failed: {e}"))?;
        println!(
            "{:<10} {:>8} {:>16} {:>10.2}",
            result.name, result.harts, result.instructions, result.mips
        );
    }

    Ok(())
}

/// Run VM in headless mode (no GUI)
fn run_headless(mut vm: NativeVm) {
    vm.run();

    // Report exit status
    let halt_code = vm.shared.halt_code();
    if halt_code == 0x5555 {
        uart_println!();
        uart_println!("[VM] Clean shutdown (PASS)");
    } else {
        uart_println!();
        uart_println!("[VM] Shutdown with code: {:#x}", halt_code);
    }
}
