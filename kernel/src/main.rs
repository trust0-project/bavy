#![no_std]
#![no_main]

mod allocator;
mod dns;
mod net;
mod uart;
mod virtio_net;
extern crate alloc;
use alloc::vec::Vec;
use panic_halt as _;
use riscv_rt::entry;

/// CLINT mtime register address (for timestamps)
const CLINT_MTIME: usize = 0x0200_BFF8;

/// Global network state (initialized lazily)
static mut NET_STATE: Option<net::NetState> = None;

/// Ping state for tracking echo requests
struct PingState {
    #[allow(dead_code)]
    target: smoltcp::wire::Ipv4Address,
    seq: u16,
    sent_time: i64,
    waiting: bool,
}

static mut PING_STATE: Option<PingState> = None;

/// Read current time in milliseconds from CLINT mtime register
fn get_time_ms() -> i64 {
    // mtime runs at 10MHz typically, convert to ms
    let mtime = unsafe { core::ptr::read_volatile(CLINT_MTIME as *const u64) };
    (mtime / 10_000) as i64
}

#[entry]
fn main() -> ! {
    uart::write_line("Booting RISC-V kernel CLI...");
    uart::write_line("Type 'help' for a list of commands.");
    // Initialize linked list allocator
    allocator::init();
    
    // Initialize network
    init_network();
    
    print_prompt();

    let console = uart::Console::new();
    let mut buffer = [0u8; 128];
    let mut len = 0usize;
    let mut count: usize = 0;

    loop {
        // Poll network stack
        poll_network();
        
        let byte = console.read_byte();

        // 0 means "no input" in our UART model
        if byte == 0 {
            continue;
        }

        match byte {
            b'\r' | b'\n' => {
                uart::write_line("");
                handle_line(&buffer, len, &mut count);
                print_prompt();
                len = 0;
            }
            // Backspace / Delete
            8 | 0x7f => {
                if len > 0 {
                    len -= 1;
                    // Move cursor back, erase char, move back again.
                    // (Simple TTY-style backspace handling.)
                    uart::write_str("\u{8} \u{8}");
                }
            }
            _ => {
                if len < buffer.len() {
                    buffer[len] = byte;
                    len += 1;
                    uart::Console::new().write_byte(byte);
                }
            }
        }
    }
}

/// Initialize the network stack
fn init_network() {
    uart::write_line("Scanning for VirtIO network device...");
    
    // Probe for VirtIO network device
    match virtio_net::VirtioNet::probe() {
        Some(device) => {
            uart::write_str("  Found at 0x");
            uart::write_hex(device.base_addr() as u64);
            uart::write_line("");
            
            match net::NetState::new(device) {
                Ok(state) => {
                    let mac = state.mac_str();
                    uart::write_line("Network initialized:");
                    uart::write_str("  MAC: ");
                    uart::write_bytes(&mac);
                    uart::write_line("");
                    uart::write_str("  IP:  ");
                    let mut ip_buf = [0u8; 16];
                    let ip_len = net::format_ipv4(net::IP_ADDR, &mut ip_buf);
                    uart::write_bytes(&ip_buf[..ip_len]);
                    uart::write_line("");
                    // Store in static FIRST, then finalize (so buffer addresses are correct)
                    unsafe { 
                        NET_STATE = Some(state);
                        // Now finalize - populates RX buffers with correct addresses
                        if let Some(ref mut s) = NET_STATE {
                            s.finalize();
                        }
                    }
                }
                Err(e) => {
                    uart::write_str("  Init FAILED: ");
                    uart::write_line(e);
                }
            }
        }
        None => {
            uart::write_line("  No VirtIO network device found.");
            uart::write_line("  Run VM with --net-tap <tapname> or --net-dummy to enable networking.");
        }
    }
}

/// Poll the network stack
fn poll_network() {
    let timestamp = get_time_ms();
    
    unsafe {
        if let Some(ref mut state) = NET_STATE {
            state.poll(timestamp);
            
            // Check for ping reply
            if let Some(ref mut ping) = PING_STATE {
                if ping.waiting {
                    if let Some((from, _ident, seq)) = state.check_ping_reply() {
                        if seq == ping.seq {
                            let rtt = timestamp - ping.sent_time;
                            uart::write_str("Reply from ");
                            let mut ip_buf = [0u8; 16];
                            let ip_len = net::format_ipv4(from, &mut ip_buf);
                            uart::write_bytes(&ip_buf[..ip_len]);
                            uart::write_str(": seq=");
                            uart::write_u64(seq as u64);
                            uart::write_str(" time=");
                            uart::write_u64(rtt as u64);
                            uart::write_line("ms");
                            ping.waiting = false;
                        }
                    }
                    
                    // Timeout after 5 seconds
                    if timestamp - ping.sent_time > 5000 {
                        uart::write_line("Request timed out");
                        ping.waiting = false;
                    }
                }
            }
        }
    }
}

fn print_prompt() {
    uart::write_str("risk-v> ");
}

fn handle_line(buffer: &[u8], len: usize, count: &mut usize) {
    // Trim leading/trailing whitespace (spaces and tabs only)
    let mut start = 0;
    let mut end = len;

    while start < end && (buffer[start] == b' ' || buffer[start] == b'\t') {
        start += 1;
    }
    while end > start && (buffer[end - 1] == b' ' || buffer[end - 1] == b'\t') {
        end -= 1;
    }

    if start >= end {
        // Empty line -> show help
        show_help();
        return;
    }

    let line = &buffer[start..end];

    // Split into command and arguments (first whitespace)
    let mut i = 0;
    while i < line.len() && line[i] != b' ' && line[i] != b'\t' {
        i += 1;
    }
    let cmd = &line[..i];

    let mut arg_start = i;
    while arg_start < line.len() && (line[arg_start] == b' ' || line[arg_start] == b'\t') {
        arg_start += 1;
    }
    let args = &line[arg_start..];

    if eq_cmd(cmd, b"help") {
        show_help();
    } else if eq_cmd(cmd, b"hello") {
        *count += 400;
        uart::write_str("Hello, count ");
        uart::write_u64(*count as u64);
        uart::write_line("");
    } else if eq_cmd(cmd, b"count") {
        uart::write_str("Current count: ");
        uart::write_u64(*count as u64);
        uart::write_line("");
    } else if eq_cmd(cmd, b"clear") {
        for _ in 0..20 {
            uart::write_line("");
        }
    } else if eq_cmd(cmd, b"echo") {
        uart::write_bytes(args);
        uart::write_line("");
    } else if eq_cmd(cmd, b"alloc") {
        cmd_alloc(args);
    } else if eq_cmd(cmd, b"memtest") {
        cmd_memtest(args);
    } else if eq_cmd(cmd, b"memstats") {
        cmd_memstats();
    } else if eq_cmd(cmd, b"ip") {
        cmd_ip(args);
    } else if eq_cmd(cmd, b"ping") {
        cmd_ping(args);
    } else if eq_cmd(cmd, b"nslookup") {
        cmd_nslookup(args);
    } else if eq_cmd(cmd, b"netstat") {
        cmd_netstat();
    } else {
        uart::write_str("Unknown command: ");
        uart::write_bytes(cmd);
        uart::write_line("");
    }
}

fn show_help() {
    uart::write_line("Available commands:");
    uart::write_line("  help           - show this help");
    uart::write_line("  hello          - increment and print the counter");
    uart::write_line("  count          - show current counter value");
    uart::write_line("  echo <text>    - print <text>");
    uart::write_line("  clear          - print a few newlines");
    uart::write_line("  alloc <bytes>  - allocate bytes (leaked) to test heap usage");
    uart::write_line("  memtest [n]    - run n allocation/deallocation cycles (default: 10)");
    uart::write_line("  memstats       - show heap memory statistics");
    uart::write_line("  ip addr        - show network interface info (MAC/IP)");
    uart::write_line("  ping <ip|host> - send ICMP echo request (resolves hostnames)");
    uart::write_line("  nslookup <host> - DNS lookup (resolve hostname to IP)");
    uart::write_line("  netstat        - show network statistics");
}

fn cmd_alloc(args: &[u8]) {
    // Parse decimal size from args
    let n = parse_usize(args);
    if n > 0 {
        // Allocate and leak
        let mut v: Vec<u8> = Vec::with_capacity(n);
        v.resize(n, 0);
        core::mem::forget(v);
        uart::write_str("Allocated ");
        uart::write_u64(n as u64);
        uart::write_line(" bytes (leaked).");
    } else {
        uart::write_line("Usage: alloc <bytes>");
    }
}

fn cmd_memtest(args: &[u8]) {
    // Parse iteration count, default to 10
    let iterations = {
        let n = parse_usize(args);
        if n == 0 { 10 } else { n }
    };

    uart::write_str("Running ");
    uart::write_u64(iterations as u64);
    uart::write_line(" memory test iterations...");

    let (used_before, free_before) = allocator::heap_stats();
    uart::write_str("  Before: used=");
    uart::write_u64(used_before as u64);
    uart::write_str(" free=");
    uart::write_u64(free_before as u64);
    uart::write_line("");

    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for i in 0..iterations {
        // Allocate a Vec, fill it with a pattern, verify, then drop
        let size = 1024; // 1KB per iteration
        let pattern = ((i % 256) as u8).wrapping_add(0x42);

        let mut v: Vec<u8> = Vec::with_capacity(size);
        v.resize(size, pattern);

        // Verify contents
        let mut ok = true;
        for &byte in v.iter() {
            if byte != pattern {
                ok = false;
                break;
            }
        }

        if ok {
            success_count += 1;
        } else {
            fail_count += 1;
        }

        // v is dropped here, memory should be freed
    }

    let (used_after, free_after) = allocator::heap_stats();
    uart::write_str("  After:  used=");
    uart::write_u64(used_after as u64);
    uart::write_str(" free=");
    uart::write_u64(free_after as u64);
    uart::write_line("");

    uart::write_str("Results: ");
    uart::write_u64(success_count as u64);
    uart::write_str(" passed, ");
    uart::write_u64(fail_count as u64);
    uart::write_line(" failed.");

    // Check if memory was properly reclaimed
    if used_after <= used_before + 64 {
        // Allow small overhead for fragmentation
        uart::write_line("Memory deallocation: OK (memory reclaimed)");
    } else {
        uart::write_line("WARNING: Memory may not be properly reclaimed!");
        uart::write_str("  Leaked approximately ");
        uart::write_u64((used_after - used_before) as u64);
        uart::write_line(" bytes");
    }
}

fn cmd_memstats() {
    let total = allocator::heap_size();
    let (used, free) = allocator::heap_stats();

    uart::write_line("Heap Memory Statistics:");
    uart::write_str("  Total:  ");
    uart::write_u64(total as u64);
    uart::write_line(" bytes");
    uart::write_str("  Used:   ");
    uart::write_u64(used as u64);
    uart::write_line(" bytes");
    uart::write_str("  Free:   ");
    uart::write_u64(free as u64);
    uart::write_line(" bytes");

    // Calculate percentage used
    if total > 0 {
        let percent_used = (used * 100) / total;
        uart::write_str("  Usage:  ");
        uart::write_u64(percent_used as u64);
        uart::write_line("%");
    }
}

fn cmd_ip(args: &[u8]) {
    // Check for "addr" subcommand
    if args.is_empty() || eq_cmd(args, b"addr") {
        unsafe {
            if let Some(ref state) = NET_STATE {
                uart::write_line("Network Interface:");
                uart::write_str("  MAC Address: ");
                uart::write_bytes(&state.mac_str());
                uart::write_line("");
                uart::write_str("  IPv4 Address: ");
                let mut ip_buf = [0u8; 16];
                let ip_len = net::format_ipv4(net::IP_ADDR, &mut ip_buf);
                uart::write_bytes(&ip_buf[..ip_len]);
                uart::write_str("/");
                uart::write_u64(net::PREFIX_LEN as u64);
                uart::write_line("");
                uart::write_str("  Gateway: ");
                let gw_len = net::format_ipv4(net::GATEWAY, &mut ip_buf);
                uart::write_bytes(&ip_buf[..gw_len]);
                uart::write_line("");
            } else {
                uart::write_line("Network not initialized");
            }
        }
    } else {
        uart::write_line("Usage: ip addr");
    }
}

fn cmd_ping(args: &[u8]) {
    if args.is_empty() {
        uart::write_line("Usage: ping <ip|hostname>");
        uart::write_line("Examples:");
        uart::write_line("  ping 10.0.2.2");
        uart::write_line("  ping google.com");
        return;
    }
    
    // Trim any trailing whitespace
    let mut arg_len = args.len();
    while arg_len > 0 && (args[arg_len - 1] == b' ' || args[arg_len - 1] == b'\t') {
        arg_len -= 1;
    }
    let trimmed_args = &args[..arg_len];
    
    // Try to parse as IP address first
    let target = match net::parse_ipv4(trimmed_args) {
        Some(ip) => ip,
        None => {
            // Not an IP address - try to resolve as hostname
            uart::write_str("Resolving ");
            uart::write_bytes(trimmed_args);
            uart::write_line("...");
            
            unsafe {
                if let Some(ref mut state) = NET_STATE {
                    match dns::resolve(state, trimmed_args, net::DNS_SERVER, 5000, get_time_ms) {
                        Some(resolved_ip) => {
                            let mut ip_buf = [0u8; 16];
                            let ip_len = net::format_ipv4(resolved_ip, &mut ip_buf);
                            uart::write_str("Resolved to ");
                            uart::write_bytes(&ip_buf[..ip_len]);
                            uart::write_line("");
                            resolved_ip
                        }
                        None => {
                            uart::write_str("Failed to resolve hostname: ");
                            uart::write_bytes(trimmed_args);
                            uart::write_line("");
                            return;
                        }
                    }
                } else {
                    uart::write_line("Network not initialized");
                    return;
                }
            }
        }
    };
    
    unsafe {
        if let Some(ref mut state) = NET_STATE {
            // Get current sequence number
            let seq = match &PING_STATE {
                Some(ps) => ps.seq.wrapping_add(1),
                None => 1,
            };
            
            let timestamp = get_time_ms();
            
            uart::write_str("PING ");
            let mut ip_buf = [0u8; 16];
            let ip_len = net::format_ipv4(target, &mut ip_buf);
            uart::write_bytes(&ip_buf[..ip_len]);
            uart::write_line("");
            
            // Set up ping state
            PING_STATE = Some(PingState {
                target,
                seq,
                sent_time: timestamp,
                waiting: true,
            });
            
            // Send the actual ICMP echo request
            match state.send_ping(target, seq, timestamp) {
                Ok(()) => {
                    uart::write_str("Sending ICMP echo request, seq=");
                    uart::write_u64(seq as u64);
                    uart::write_line("...");
                }
                Err(e) => {
                    uart::write_str("Failed to send ping: ");
                    uart::write_line(e);
                    PING_STATE = None;
                }
            }
        } else {
            uart::write_line("Network not initialized");
        }
    }
}

fn cmd_nslookup(args: &[u8]) {
    if args.is_empty() {
        uart::write_line("Usage: nslookup <hostname>");
        uart::write_line("Example: nslookup google.com");
        return;
    }
    
    // Trim any trailing whitespace from hostname
    let mut hostname_len = args.len();
    while hostname_len > 0 && (args[hostname_len - 1] == b' ' || args[hostname_len - 1] == b'\t') {
        hostname_len -= 1;
    }
    let hostname = &args[..hostname_len];
    
    unsafe {
        if let Some(ref mut state) = NET_STATE {
            uart::write_str("Looking up ");
            uart::write_bytes(hostname);
            uart::write_line("...");
            
            uart::write_str("Server: ");
            let mut ip_buf = [0u8; 16];
            let dns_len = net::format_ipv4(net::DNS_SERVER, &mut ip_buf);
            uart::write_bytes(&ip_buf[..dns_len]);
            uart::write_line("");
            uart::write_line("");
            
            // Perform DNS lookup with 5 second timeout
            match dns::resolve(state, hostname, net::DNS_SERVER, 5000, get_time_ms) {
                Some(addr) => {
                    uart::write_str("Name:    ");
                    uart::write_bytes(hostname);
                    uart::write_line("");
                    uart::write_str("Address: ");
                    let addr_len = net::format_ipv4(addr, &mut ip_buf);
                    uart::write_bytes(&ip_buf[..addr_len]);
                    uart::write_line("");
                }
                None => {
                    uart::write_str("*** Can't find ");
                    uart::write_bytes(hostname);
                    uart::write_line(": No response from server");
                }
            }
        } else {
            uart::write_line("Network not initialized");
        }
    }
}

fn cmd_netstat() {
    unsafe {
        if let Some(ref _state) = NET_STATE {
            uart::write_line("Network Status:");
            uart::write_str("  Device: VirtIO-Net at 0x");
            uart::write_hex(virtio_net::VIRTIO_NET_BASE as u64);
            uart::write_line("");
            uart::write_str("  Status: ");
            uart::write_line("UP");
        } else {
            uart::write_line("Network not initialized");
        }
    }
}

fn parse_usize(args: &[u8]) -> usize {
    let mut n: usize = 0;
    let mut ok = false;
    for &b in args {
        if b >= b'0' && b <= b'9' {
            ok = true;
            let d = (b - b'0') as usize;
            n = n.saturating_mul(10).saturating_add(d);
        } else if b == b' ' || b == b'\t' {
            if ok {
                break;
            }
        } else {
            break;
        }
    }
    if ok { n } else { 0 }
}

fn eq_cmd(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
