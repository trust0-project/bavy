use alloc::{string::String, vec::Vec};
use core::ptr;
use core::sync::atomic::Ordering;

use crate::{
    allocator, dns, net, scripting, uart, BenchmarkMode, PingState, BENCHMARK, BLK_DEV,
    COMMAND_RUNNING, FS_STATE, HARTS_ONLINE, NET_STATE, PING_STATE, TEST_FINISHER,
};
use crate::{count_primes_in_range, cwd_set, get_time_ms, resolve_path, send_ipi};
use crate::{out_line, out_str};

pub fn node(args: &[u8]) {
    let args_str = core::str::from_utf8(args).unwrap_or("").trim();

    if args_str.is_empty() || args_str == "info" {
        scripting::print_info();
    } else if args_str.starts_with("log ") {
        let level_str = args_str.strip_prefix("log ").unwrap_or("").trim();
        let level = match level_str {
            "off" | "OFF" => scripting::LogLevel::Off,
            "error" | "ERROR" => scripting::LogLevel::Error,
            "warn" | "WARN" => scripting::LogLevel::Warn,
            "info" | "INFO" => scripting::LogLevel::Info,
            "debug" | "DEBUG" => scripting::LogLevel::Debug,
            "trace" | "TRACE" => scripting::LogLevel::Trace,
            _ => {
                out_line("Usage: node log <level>");
                out_line("Levels: off, error, warn, info, debug, trace");
                return;
            }
        };
        scripting::set_log_level(level);
        out_str("\x1b[1;32m✓\x1b[0m Script log level set to: ");
        out_line(level_str);
    } else if args_str == "eval" || args_str.starts_with("eval ") {
        let expr = args_str.strip_prefix("eval").unwrap_or("").trim();
        if expr.is_empty() {
            out_line("Usage: node eval <expression>");
            out_line("Example: node eval 2 + 2 * 3");
            return;
        }
        match scripting::execute_script_uncached(expr, "") {
            Ok(output) => {
                if !output.is_empty() {
                    out_str(&output);
                }
            }
            Err(e) => {
                out_str("\x1b[1;31mError:\x1b[0m ");
                out_line(&e);
            }
        }
    } else if !args_str.is_empty() {
        let (script_name, script_args) = match args_str.split_once(' ') {
            Some((name, rest)) => (name, rest),
            None => (args_str, ""),
        };

        let resolved_path = if script_name.starts_with('/') {
            String::from(script_name)
        } else {
            resolve_path(script_name)
        };

        let script_result = {
            let fs_guard = FS_STATE.lock();
            let mut blk_guard = BLK_DEV.lock();
            if let (Some(fs), Some(dev)) = (fs_guard.as_ref(), blk_guard.as_mut()) {
                fs.read_file(dev, &resolved_path)
            } else {
                out_line("\x1b[1;31mError:\x1b[0m Filesystem not available");
                return;
            }
        };

        match script_result {
            Some(script_bytes) => {
                if let Ok(script) = core::str::from_utf8(&script_bytes) {
                    match scripting::execute_script(script, script_args) {
                        Ok(output) => {
                            if !output.is_empty() {
                                out_str(&output);
                            }
                        }
                        Err(e) => {
                            out_str("\x1b[1;31mScript error:\x1b[0m ");
                            out_line(&e);
                        }
                    }
                } else {
                    out_line("\x1b[1;31mError:\x1b[0m Invalid UTF-8 in script file");
                }
            }
            None => {
                out_str("\x1b[1;31mError:\x1b[0m Script not found: ");
                out_line(&resolved_path);
            }
        }
    }
}

pub fn help() {
    out_line("\x1b[1;36m┌─────────────────────────────────────────────────────────────┐\x1b[0m");
    out_line(
        "\x1b[1;36m│\x1b[0m                   \x1b[1;97mBAVY OS Commands\x1b[0m                        \x1b[1;36m│\x1b[0m",
    );
    out_line("\x1b[1;36m├─────────────────────────────────────────────────────────────┤\x1b[0m");
    out_line(
        "\x1b[1;36m│\x1b[0m  \x1b[1;33mBuilt-in:\x1b[0m                                                 \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    cd <dir>        Change directory                         \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    pwd             Print working directory                  \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    clear           Clear the screen                         \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    shutdown        Power off the system                     \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    ping <host>     Ping host (Ctrl+C to stop)               \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    nslookup <host> DNS lookup                               \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    node [info]     Scripting engine info/control            \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m                                                             \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m  \x1b[1;33mUser Scripts:\x1b[0m  \x1b[0;90m(in /usr/bin/ - Rhai language)\x1b[0m            \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    help, ls, cat, echo, cowsay, sysinfo, ip, memstats, ...  \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m                                                             \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m  \x1b[1;33mKernel API:\x1b[0m  \x1b[0;90m(available in scripts)\x1b[0m                      \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    ls(), read_file(), write_file(), file_exists()           \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    get_ip(), get_mac(), get_gateway(), net_available()      \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    time_ms(), sleep(ms), kernel_version(), arch()           \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m    heap_total(), heap_used(), heap_free()                   \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m                                                             \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m  \x1b[1;33mRedirection:\x1b[0m  cmd > file | cmd >> file                    \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m                                                             \x1b[1;36m│\x1b[0m",
    );
    out_line(
        "\x1b[1;36m│\x1b[0m  \x1b[1;32mTip:\x1b[0m  \x1b[1;97mCtrl+C\x1b[0m cancel  |  \x1b[1;97m↑/↓\x1b[0m history  |  \x1b[1;97mnode info\x1b[0m API  \x1b[1;36m│\x1b[0m",
    );
    out_line("\x1b[1;36m└─────────────────────────────────────────────────────────────┘\x1b[0m");
}

pub fn alloc(args: &[u8]) {
    let n = parse_usize(args);
    if n > 0 {
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

pub fn readsec(args: &[u8]) {
    let sector = parse_usize(args) as u64;
    let mut blk_guard = BLK_DEV.lock();
    if let Some(ref mut blk) = *blk_guard {
        let mut buf = [0u8; 512];
        if blk.read_sector(sector, &mut buf).is_ok() {
            uart::write_line("Sector contents (first 64 bytes):");
            for i in 0..64 {
                uart::write_hex_byte(buf[i]);
                if (i + 1) % 16 == 0 {
                    uart::write_line("");
                } else {
                    uart::write_str(" ");
                }
            }
        } else {
            uart::write_line("Read failed.");
        }
    } else {
        uart::write_line("No block device.");
    }
}

pub fn memtest(args: &[u8]) {
    let iterations = {
        let n = parse_usize(args);
        if n == 0 {
            10
        } else {
            n
        }
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
        let size = 1024;
        let pattern = ((i % 256) as u8).wrapping_add(0x42);

        let mut v: Vec<u8> = Vec::with_capacity(size);
        v.resize(size, pattern);

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

    if used_after <= used_before + 64 {
        uart::write_line("Memory deallocation: OK (memory reclaimed)");
    } else {
        uart::write_line("WARNING: Memory may not be properly reclaimed!");
        uart::write_str("  Leaked approximately ");
        uart::write_u64((used_after - used_before) as u64);
        uart::write_line(" bytes");
    }
}

pub fn cputest(args: &[u8]) {
    let limit = {
        let n = parse_usize(args);
        if n == 0 {
            100_000
        } else {
            n
        }
    };

    let num_harts = HARTS_ONLINE.load(Ordering::Relaxed);

    uart::write_line("");
    uart::write_line(
        "\x1b[1;36m╔═══════════════════════════════════════════════════════════════════════╗\x1b[0m",
    );
    uart::write_line(
        "\x1b[1;36m║\x1b[0m                      \x1b[1;97mCPU BENCHMARK - Prime Counting\x1b[0m                  \x1b[1;36m║\x1b[0m",
    );
    uart::write_line(
        "\x1b[1;36m╚═══════════════════════════════════════════════════════════════════════╝\x1b[0m",
    );
    uart::write_line("");

    uart::write_str("  \x1b[1;33mConfiguration:\x1b[0m");
    uart::write_line("");
    uart::write_str("    Range: 2 to ");
    uart::write_u64(limit as u64);
    uart::write_line("");
    uart::write_str("    Harts online: ");
    uart::write_u64(num_harts as u64);
    uart::write_line("");
    uart::write_line("");

    uart::write_line("  \x1b[1;33m[1/2] Serial Execution\x1b[0m (single hart)");
    uart::write_str("        Computing primes...");

    let serial_start = get_time_ms();
    let serial_count = count_primes_in_range(2, limit as u64);
    let serial_end = get_time_ms();
    let serial_time = serial_end - serial_start;

    uart::write_line(" done!");
    uart::write_str("        Result: \x1b[1;97m");
    uart::write_u64(serial_count);
    uart::write_str("\x1b[0m primes found in \x1b[1;97m");
    uart::write_u64(serial_time as u64);
    uart::write_line("\x1b[0m ms");
    uart::write_line("");

    if num_harts > 1 {
        uart::write_str("  \x1b[1;33m[2/2] Parallel Execution\x1b[0m (");
        uart::write_u64(num_harts as u64);
        uart::write_line(" harts)");
        uart::write_str("        Computing primes...");

        let parallel_start = get_time_ms();

        BENCHMARK.start(BenchmarkMode::PrimeCount, 2, limit as u64, num_harts);

        for hart in 1..num_harts {
            send_ipi(hart);
        }

        let (my_start, my_end) = BENCHMARK.get_work_range(0);
        let my_count = count_primes_in_range(my_start, my_end);
        BENCHMARK.report_result(0, my_count);

        let timeout = get_time_ms() + 60000;
        while !BENCHMARK.all_completed() {
            if get_time_ms() > timeout {
                uart::write_line(" TIMEOUT!");
                uart::write_line(
                    "        \x1b[1;31mError:\x1b[0m Some harts did not complete in time",
                );
                BENCHMARK.clear();
                return;
            }
            core::hint::spin_loop();
        }

        let parallel_end = get_time_ms();
        let parallel_time = parallel_end - parallel_start;
        let parallel_count = BENCHMARK.total_result();

        BENCHMARK.clear();

        uart::write_line(" done!");
        uart::write_str("        Result: \x1b[1;97m");
        uart::write_u64(parallel_count);
        uart::write_str("\x1b[0m primes found in \x1b[1;97m");
        uart::write_u64(parallel_time as u64);
        uart::write_line("\x1b[0m ms");

        uart::write_line("");
        uart::write_line("        \x1b[0;90mWork distribution:\x1b[0m");
        let chunk = (limit as u64 - 2) / num_harts as u64;
        for hart in 0..num_harts {
            let h_start = 2 + hart as u64 * chunk;
            let h_end = if hart == num_harts - 1 {
                limit as u64
            } else {
                h_start + chunk
            };
            uart::write_str("          Hart ");
            uart::write_u64(hart as u64);
            uart::write_str(": [");
            uart::write_u64(h_start);
            uart::write_str(", ");
            uart::write_u64(h_end);
            uart::write_line(")");
        }
        uart::write_line("");

        uart::write_line(
            "\x1b[1;36m────────────────────────────────────────────────────────────────────────\x1b[0m",
        );
        uart::write_line("  \x1b[1;33mResults Summary:\x1b[0m");
        uart::write_line("");

        if serial_count == parallel_count {
            uart::write_line("    \x1b[1;32m✓\x1b[0m Results match (verified correctness)");
        } else {
            uart::write_line("    \x1b[1;31m✗\x1b[0m Results MISMATCH (bug detected!)");
            uart::write_str("      Serial: ");
            uart::write_u64(serial_count);
            uart::write_str(", Parallel: ");
            uart::write_u64(parallel_count);
            uart::write_line("");
        }
        uart::write_line("");

        if parallel_time > 0 {
            let speedup_x10 = (serial_time * 10) / parallel_time;
            let speedup_whole = speedup_x10 / 10;
            let speedup_frac = speedup_x10 % 10;

            uart::write_str("    Serial time:   \x1b[1;97m");
            uart::write_u64(serial_time as u64);
            uart::write_line(" ms\x1b[0m");
            uart::write_str("    Parallel time: \x1b[1;97m");
            uart::write_u64(parallel_time as u64);
            uart::write_line(" ms\x1b[0m");
            uart::write_str("    Speedup:       \x1b[1;32m");
            uart::write_u64(speedup_whole as u64);
            uart::write_str(".");
            uart::write_u64(speedup_frac as u64);
            uart::write_str("x\x1b[0m (with ");
            uart::write_u64(num_harts as u64);
            uart::write_line(" harts)");

            let efficiency = (speedup_x10 * 100) / (num_harts as i64 * 10);
            uart::write_str("    Efficiency:    \x1b[1;97m");
            uart::write_u64(efficiency as u64);
            uart::write_line("%\x1b[0m (speedup / num_harts × 100)");
        }
        uart::write_line("");
    } else {
        uart::write_line("  \x1b[1;33m[2/2] Parallel Execution\x1b[0m");
        uart::write_line("        \x1b[0;90mSkipped - only 1 hart online\x1b[0m");
        uart::write_line("");
        uart::write_line(
            "\x1b[1;36m────────────────────────────────────────────────────────────────────────\x1b[0m",
        );
        uart::write_line("  \x1b[1;33mResults Summary:\x1b[0m");
        uart::write_line("");
        uart::write_str("    Serial time: \x1b[1;97m");
        uart::write_u64(serial_time as u64);
        uart::write_line(" ms\x1b[0m");
        uart::write_str("    Primes found: \x1b[1;97m");
        uart::write_u64(serial_count);
        uart::write_line("\x1b[0m");
        uart::write_line("");
        uart::write_line("    \x1b[0;90mNote: Enable more harts to see parallel comparison\x1b[0m");
        uart::write_line("");
    }

    uart::write_line(
        "\x1b[1;36m════════════════════════════════════════════════════════════════════════\x1b[0m",
    );
    uart::write_line("");
}

pub fn ping(args: &[u8]) {
    if args.is_empty() {
        uart::write_line("Usage: ping <ip|hostname>");
        uart::write_line("\x1b[0;90mExamples:\x1b[0m");
        uart::write_line("  ping 10.0.2.2");
        uart::write_line("  ping google.com");
        uart::write_line("\x1b[0;90mPress Ctrl+C to stop\x1b[0m");
        return;
    }

    let mut arg_len = args.len();
    while arg_len > 0 && (args[arg_len - 1] == b' ' || args[arg_len - 1] == b'\t') {
        arg_len -= 1;
    }
    let trimmed_args = &args[..arg_len];

    let target = match net::parse_ipv4(trimmed_args) {
        Some(ip) => ip,
        None => {
            uart::write_str("\x1b[0;90m[DNS]\x1b[0m Resolving ");
            uart::write_bytes(trimmed_args);
            uart::write_line("...");

            let resolve_result = {
                let mut net_guard = NET_STATE.lock();
                if let Some(ref mut state) = *net_guard {
                    dns::resolve(state, trimmed_args, net::DNS_SERVER, 5000, get_time_ms)
                } else {
                    uart::write_line("\x1b[1;31m✗\x1b[0m Network not initialized");
                    return;
                }
            };

            match resolve_result {
                Some(resolved_ip) => {
                    let mut ip_buf = [0u8; 16];
                    let ip_len = net::format_ipv4(resolved_ip, &mut ip_buf);
                    uart::write_str("\x1b[1;32m[DNS]\x1b[0m Resolved to \x1b[1;97m");
                    uart::write_bytes(&ip_buf[..ip_len]);
                    uart::write_line("\x1b[0m");
                    resolved_ip
                }
                None => {
                    uart::write_str("\x1b[1;31m[DNS]\x1b[0m Failed to resolve: ");
                    uart::write_bytes(trimmed_args);
                    uart::write_line("");
                    return;
                }
            }
        }
    };

    let timestamp = get_time_ms();

    let mut ip_buf = [0u8; 16];
    let ip_len = net::format_ipv4(target, &mut ip_buf);
    uart::write_str("PING ");
    uart::write_bytes(&ip_buf[..ip_len]);
    uart::write_line(" 56(84) bytes of data.");

    let mut ping_state = PingState::new(target, timestamp);
    ping_state.seq = 1;
    ping_state.sent_time = timestamp;
    ping_state.last_send_time = timestamp;
    ping_state.packets_sent = 1;
    ping_state.waiting = true;

    let send_result = {
        let mut net_guard = NET_STATE.lock();
        if let Some(ref mut state) = *net_guard {
            state.send_ping(target, ping_state.seq, timestamp)
        } else {
            uart::write_line("\x1b[1;31m✗\x1b[0m Network not initialized");
            return;
        }
    };

    match send_result {
        Ok(()) => {
            *PING_STATE.lock() = Some(ping_state);
            *COMMAND_RUNNING.lock() = true;
        }
        Err(e) => {
            uart::write_str("ping: ");
            uart::write_line(e);
        }
    }
}

pub fn nslookup(args: &[u8]) {
    if args.is_empty() {
        uart::write_line("Usage: nslookup <hostname>");
        uart::write_line("\x1b[0;90mExample: nslookup google.com\x1b[0m");
        return;
    }

    let mut hostname_len = args.len();
    while hostname_len > 0 && (args[hostname_len - 1] == b' ' || args[hostname_len - 1] == b'\t') {
        hostname_len -= 1;
    }
    let hostname = &args[..hostname_len];

    uart::write_line("");
    uart::write_str("\x1b[1;33mServer:\x1b[0m  ");
    let mut ip_buf = [0u8; 16];
    let dns_len = net::format_ipv4(net::DNS_SERVER, &mut ip_buf);
    uart::write_bytes(&ip_buf[..dns_len]);
    uart::write_line("");
    uart::write_line("\x1b[1;33mPort:\x1b[0m    53");
    uart::write_line("");

    uart::write_str("\x1b[0;90mQuerying ");
    uart::write_bytes(hostname);
    uart::write_line("...\x1b[0m");

    let resolve_result = {
        let mut net_guard = NET_STATE.lock();
        if let Some(ref mut state) = *net_guard {
            dns::resolve(state, hostname, net::DNS_SERVER, 5000, get_time_ms)
        } else {
            uart::write_line("\x1b[1;31m✗\x1b[0m Network not initialized");
            return;
        }
    };

    match resolve_result {
        Some(addr) => {
            uart::write_line("");
            uart::write_str("\x1b[1;32mName:\x1b[0m    ");
            uart::write_bytes(hostname);
            uart::write_line("");
            let addr_len = net::format_ipv4(addr, &mut ip_buf);
            uart::write_str("\x1b[1;32mAddress:\x1b[0m \x1b[1;97m");
            uart::write_bytes(&ip_buf[..addr_len]);
            uart::write_line("\x1b[0m");
            uart::write_line("");
        }
        None => {
            uart::write_line("");
            uart::write_str("\x1b[1;31m*** Can't find ");
            uart::write_bytes(hostname);
            uart::write_line(": No response from server\x1b[0m");
            uart::write_line("");
        }
    }
}

pub fn cd(args: &str) {
    let path = args.trim();

    if path.is_empty() || path == "~" {
        cwd_set("/");
        return;
    }

    if path == "-" {
        out_line("cd: OLDPWD not set");
        return;
    }

    let new_path = resolve_path(path);

    if path_exists(&new_path) {
        cwd_set(&new_path);
    } else {
        out_str("\x1b[1;31mcd:\x1b[0m ");
        out_str(path);
        out_line(": No such directory");
    }
}

pub fn shutdown() {
    uart::write_line("");
    uart::write_line(
        "\x1b[1;31m╔═══════════════════════════════════════════════════════════════════╗\x1b[0m",
    );
    uart::write_line(
        "\x1b[1;31m║\x1b[0m                                                                   \x1b[1;31m║\x1b[0m",
    );
    uart::write_line(
        "\x1b[1;31m║\x1b[0m                    \x1b[1;97mSystem Shutdown Initiated\x1b[0m                       \x1b[1;31m║\x1b[0m",
    );
    uart::write_line(
        "\x1b[1;31m║\x1b[0m                                                                   \x1b[1;31m║\x1b[0m",
    );
    uart::write_line(
        "\x1b[1;31m╚═══════════════════════════════════════════════════════════════════╝\x1b[0m",
    );
    uart::write_line("");
    uart::write_line("    \x1b[0;90m[1/3]\x1b[0m Syncing filesystems...");
    uart::write_line("    \x1b[0;90m[2/3]\x1b[0m Stopping network services...");
    uart::write_line("    \x1b[0;90m[3/3]\x1b[0m Powering off CPU...");
    uart::write_line("");
    uart::write_line("    \x1b[1;32m✓ Goodbye!\x1b[0m");
    uart::write_line("");

    unsafe {
        ptr::write_volatile(TEST_FINISHER as *mut u32, 0x5555);
    }
    loop {}
}

fn parse_usize(args: &[u8]) -> usize {
    let mut n: usize = 0;
    let mut ok = false;
    for &b in args {
        if (b'0'..=b'9').contains(&b) {
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
    if ok {
        n
    } else {
        0
    }
}

fn path_exists(path: &str) -> bool {
    let fs_guard = FS_STATE.lock();
    let mut blk_guard = BLK_DEV.lock();
    if let (Some(fs), Some(dev)) = (fs_guard.as_ref(), blk_guard.as_mut()) {
        if path == "/" {
            return true;
        }

        let files = fs.list_dir(dev, "/");
        let path_with_slash = if path.ends_with('/') {
            String::from(path)
        } else {
            let mut s = String::from(path);
            s.push('/');
            s
        };

        for file in files {
            if file.name.starts_with(&path_with_slash) {
                return true;
            }
            if file.name == path {
                return true;
            }
        }
    }
    false
}
