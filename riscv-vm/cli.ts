#!/usr/bin/env node
/// <reference types="node" />
/**
 * RISC-V VM CLI
 * This CLI mirrors the native Rust VM CLI interface:
 * - loads an SD card image via --sdcard/-s (contains kernel + filesystem)
 * - optionally specifies number of harts via --harts/-n (0 = auto-detect as CPU/2)
 * - can optionally connect to a network relay via --net-webtransport
 * - runs the VM in a tight loop
 * - connects stdin → UART input and UART output → stdout
 * 
 * Multi-hart support:
 * - Uses Node.js worker_threads for parallel execution
 * - Hart 0 runs on main thread (handles I/O)
 * - Harts 1+ run on worker threads
 * - All harts share memory via SharedArrayBuffer
 */

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Worker } from 'node:worker_threads';
import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';

// Default relay server URL and cert hash.
const DEFAULT_RELAY_URL =
  process.env.RELAY_URL || 'https://localhost:4433';
const DEFAULT_CERT_HASH =
  process.env.RELAY_CERT_HASH || '';

/**
 * Auto-detect the number of harts based on CPU cores.
 * Uses all available cores since idle harts sleep via WFI (no CPU waste).
 */
function detectHartCount(): number {
  return os.cpus().length;
}

/**
 * SD Card boot info parsed from MBR + FAT32
 */
interface SDBootInfo {
  kernelData: Uint8Array;
  fsPartitionStart: number;
}

/**
 * Parse SD card image (MBR + FAT32 boot partition)
 * Returns kernel data and filesystem partition offset
 */
function parseSDCard(sdcard: Uint8Array): SDBootInfo {
  if (sdcard.length < 512) {
    throw new Error('SD card image too small');
  }

  // Check MBR signature
  if (sdcard[510] !== 0x55 || sdcard[511] !== 0xAA) {
    throw new Error('Invalid MBR signature');
  }

  // Parse partition table (4 entries at offset 446)
  let bootPartStart = 0;
  let bootPartType = 0;
  let fsPartStart = 0;

  for (let i = 0; i < 4; i++) {
    const offset = 446 + i * 16;
    const partType = sdcard[offset + 4];
    const startLba = sdcard[offset + 8] | (sdcard[offset + 9] << 8) |
      (sdcard[offset + 10] << 16) | (sdcard[offset + 11] << 24);

    // FAT32: 0x0B (CHS) or 0x0C (LBA)
    if ((partType === 0x0B || partType === 0x0C) && bootPartStart === 0) {
      bootPartStart = startLba;
      bootPartType = partType;
    } else if (partType !== 0 && fsPartStart === 0 && startLba !== bootPartStart) {
      fsPartStart = startLba;
    }
  }

  if (bootPartStart === 0) {
    throw new Error('No FAT32 boot partition found');
  }

  // Parse FAT32 boot sector
  const bootSectorOffset = bootPartStart * 512;
  if (bootSectorOffset + 512 > sdcard.length) {
    throw new Error('Boot partition beyond disk');
  }

  const bytesPerSector = sdcard[bootSectorOffset + 11] | (sdcard[bootSectorOffset + 12] << 8);
  const sectorsPerCluster = sdcard[bootSectorOffset + 13];
  const reservedSectors = sdcard[bootSectorOffset + 14] | (sdcard[bootSectorOffset + 15] << 8);
  const numFats = sdcard[bootSectorOffset + 16];
  const sectorsPerFat = sdcard[bootSectorOffset + 36] | (sdcard[bootSectorOffset + 37] << 8) |
    (sdcard[bootSectorOffset + 38] << 16) | (sdcard[bootSectorOffset + 39] << 24);
  const rootCluster = sdcard[bootSectorOffset + 44] | (sdcard[bootSectorOffset + 45] << 8) |
    (sdcard[bootSectorOffset + 46] << 16) | (sdcard[bootSectorOffset + 47] << 24);

  // Calculate data start sector
  const dataStartSector = reservedSectors + (numFats * sectorsPerFat);
  const rootDirSector = dataStartSector + (rootCluster - 2) * sectorsPerCluster;
  const rootDirOffset = bootSectorOffset + (rootDirSector * bytesPerSector);

  // Search for KERNEL.BIN in root directory
  const clusterBytes = sectorsPerCluster * bytesPerSector;
  let kernelData: Uint8Array | null = null;

  for (let i = 0; i < clusterBytes; i += 32) {
    const entryOffset = rootDirOffset + i;
    if (entryOffset + 32 > sdcard.length) break;

    const firstByte = sdcard[entryOffset];
    if (firstByte === 0x00) break; // End of directory
    if (firstByte === 0xE5) continue; // Deleted
    if (sdcard[entryOffset + 11] === 0x0F) continue; // Long name entry

    // Check for KERNEL  BIN (8.3 format, space-padded)
    const name = String.fromCharCode(...sdcard.slice(entryOffset, entryOffset + 11));
    if (name === 'KERNEL  BIN') {
      const clusterHigh = sdcard[entryOffset + 20] | (sdcard[entryOffset + 21] << 8);
      const clusterLow = sdcard[entryOffset + 26] | (sdcard[entryOffset + 27] << 8);
      const fileSize = sdcard[entryOffset + 28] | (sdcard[entryOffset + 29] << 8) |
        (sdcard[entryOffset + 30] << 16) | (sdcard[entryOffset + 31] << 24);

      const fileCluster = (clusterHigh << 16) | clusterLow;
      const fileSector = dataStartSector + (fileCluster - 2) * sectorsPerCluster;
      const fileOffset = bootSectorOffset + (fileSector * bytesPerSector);

      if (fileOffset + fileSize <= sdcard.length) {
        kernelData = sdcard.slice(fileOffset, fileOffset + fileSize);
      }
      break;
    }
  }

  if (!kernelData) {
    throw new Error('kernel.bin not found on boot partition');
  }

  return {
    kernelData,
    fsPartitionStart: fsPartStart,
  };
}

/**
 * Try to load the native WebTransport addon.
 * Returns the WebTransportClient class if available, null otherwise.
 */
async function loadNativeWebTransport(): Promise<any | null> {
  // Try to load from the native directory (built with npm run build:native)
  // ESM requires explicit file path, not directory imports
  const addonPath = path.resolve(__dirname, '..', 'native', 'index.js');

  try {
    const addon = await import(addonPath);
    if (addon.WebTransportClient) {
      console.error('[CLI] Native WebTransport addon loaded');
      return addon.WebTransportClient;
    }
    console.error('[CLI] Native addon loaded but WebTransportClient not found');
  } catch (e: any) {
    // Not available - likely not built yet
    console.error('[CLI] Native WebTransport addon not available');
    console.error(`[CLI] Tried to load from: ${addonPath}`);
    console.error(`[CLI] Error: ${e.message || e}`);
    console.error('[CLI] Build it with: cd riscv-vm && npm run build:native');
  }
  return null;
}

/**
 * Create and initialize a Wasm VM instance with multi-hart support:
 * - loads SD card image and parses MBR/FAT32 to find kernel
 * - constructs `WasmVm` with the kernel bytes and specified hart count
 * - loads entire SD card as block device (for filesystem partition)
 * - optionally connects to a network relay (WebTransport/WebSocket)
 * - starts worker threads for secondary harts
 */
async function createVm(
  sdcardPath: string,
  options?: {
    harts?: number;
    netWebtransport?: string;
    certHash?: string;
    debug?: boolean;
    enableGpu?: boolean;
  },
) {
  let sdcardBytes: Uint8Array;

  // Load SD card image
  if (sdcardPath.startsWith('http://') || sdcardPath.startsWith('https://')) {
    if (options?.debug) {
      console.error(`[CLI] Downloading SD card from ${sdcardPath}...`);
    }
    const response = await fetch(sdcardPath);
    if (!response.ok) {
      throw new Error(
        `Failed to fetch SD card from ${sdcardPath}: ${response.statusText}`,
      );
    }
    const arrayBuffer = await response.arrayBuffer();
    sdcardBytes = new Uint8Array(arrayBuffer);
  } else {
    const resolvedPath = path.resolve(sdcardPath);
    if (!fs.existsSync(resolvedPath)) {
      throw new Error(`SD card image not found at ${resolvedPath}`);
    }
    const sdcardBuf = fs.readFileSync(resolvedPath);
    sdcardBytes = new Uint8Array(sdcardBuf);
  }

  // Parse SD card to find kernel on boot partition
  const bootInfo = parseSDCard(sdcardBytes);
  const kernelBytes = bootInfo.kernelData;

  if (options?.debug) {
    console.error(`[CLI] Found kernel: ${kernelBytes.length} bytes`);
    console.error(`[CLI] Filesystem partition at sector ${bootInfo.fsPartitionStart}`);
  }

  const { WasmInternal } = await import('./');
  const wasm = await WasmInternal();
  const VmCtor = wasm.WasmVm;
  if (!VmCtor) {
    throw new Error('WasmVm class not found in wasm module');
  }

  // Create VM with requested number of harts
  const requestedHarts = options?.harts;
  const vm = (requestedHarts !== undefined && requestedHarts >= 1 && VmCtor.new_with_harts)
    ? VmCtor.new_with_harts(kernelBytes, requestedHarts)
    : new VmCtor(kernelBytes);

  // Load entire SD card as block device (for filesystem partition access)
  if (typeof vm.load_disk === 'function') {
    vm.load_disk(sdcardBytes);
    if (options?.debug) {
      console.error(`[VM] Loaded SD card as block device`);
    }
  }

  // Enable GPU rendering if requested
  if (options?.enableGpu && typeof vm.enable_gpu === 'function') {
    vm.enable_gpu(1024, 768);
    console.error(`[VM] GPU enabled (1024x768)`);
  }

  // Start worker threads for secondary harts (1..numHarts)
  // Use vm.num_harts() to get the actual hart count (handles auto-detect case)
  const workers: Worker[] = [];
  const actualHarts = typeof vm.num_harts === 'function' ? vm.num_harts() : (requestedHarts ?? 1);
  if (actualHarts > 1 && typeof vm.get_shared_buffer === 'function') {
    const sharedBuffer = vm.get_shared_buffer();

    if (sharedBuffer) {
      // Get entry PC for workers
      const entryPc = typeof (vm as any).entry_pc === 'function' ? (vm as any).entry_pc() : 0x80000000;

      // Path to worker script - WASM bytes are passed directly
      const workerPath = path.resolve(__dirname, 'node-worker.js');

      console.error(`[VM] Starting ${actualHarts - 1} worker threads...`);

      // Track ready workers - allow start only after ALL workers are ready
      // This fixes: with 9+ harts, workers take longer to spawn than the 100ms timeout
      let readyWorkers = 0;
      const expectedWorkers = actualHarts - 1;

      for (let hartId = 1; hartId < actualHarts; hartId++) {
        const worker = new Worker(workerPath, {
          workerData: {
            hartId,
            sharedMem: sharedBuffer,
            entryPc: Number(entryPc),
          },
        });

        worker.on('message', (msg: any) => {
          if (msg.type === 'ready') {
            console.error(`[VM] Worker ${msg.hartId} ready`);
            readyWorkers++;
            // When all workers are ready AND main thread says it's OK, allow workers to start
            // The workers are blocked in wait_brief() until we set the start signal
            if (readyWorkers === expectedWorkers) {
              // Give a tiny bit more time for hart 0 to get ahead in boot
              setTimeout(() => {
                if (typeof (vm as any).allow_workers_to_start === 'function') {
                  (vm as any).allow_workers_to_start();
                  // Note: wasm.rs logs 'Workers signaled to start', we log 'allowed' here
                }
              }, 10);
            }
          } else if (msg.type === 'halted') {
            console.error(`[VM] Worker ${msg.hartId} halted (${msg.stepCount} steps)`);
          } else if (msg.type === 'error') {
            console.error(`[VM] Worker ${msg.hartId} error: ${msg.error}`);
          }
        });

        worker.on('error', (err) => {
          console.error(`[VM] Worker ${hartId} error:`, err);
        });

        worker.on('exit', (code) => {
          if (code !== 0) {
            console.error(`[VM] Worker ${hartId} exited with code ${code}`);
          }
        });

        workers.push(worker);
      }

      console.error(`[VM] Started ${workers.length} worker threads`);
    } else {
      console.error('[VM] Warning: SharedArrayBuffer not available, running single-threaded');
    }
  }

  // Network setup via native WebTransport addon (Node.js)
  let nativeNetClient: any = null;
  if (options?.netWebtransport) {
    const relayUrl = options.netWebtransport;
    const certHash = options.certHash || DEFAULT_CERT_HASH || undefined;

    // Try to use native WebTransport addon
    const WebTransportClient = await loadNativeWebTransport();

    if (WebTransportClient) {
      // Use native addon for WebTransport
      nativeNetClient = new WebTransportClient(relayUrl, certHash);

      // Get MAC address from native client and set up external network
      const macBytes = nativeNetClient.macBytes();
      if (typeof vm.setup_external_network === 'function') {
        vm.setup_external_network(new Uint8Array(macBytes));
      } else {
        nativeNetClient.shutdown();
        nativeNetClient = null;
      }
    }
  }

  return { vm, nativeNetClient, workers };
}

/**
 * Run the VM in a loop and wire stdin/stdout to the UART, similar to the browser loop:
 * - executes a fixed number of instructions per tick
 * - drains the UART output buffer and writes to stdout
 * - feeds raw stdin bytes into the VM's UART input
 * - bridges packets between native WebTransport addon and VM
 * - manages worker threads for secondary harts
 */
function runVmLoop(vm: any, nativeNetClient: any | null, workers: Worker[] = []) {
  let running = true;
  let networkConnected = false;

  const shutdown = (code: number) => {
    if (!running) return;
    running = false;

    // Terminate worker threads
    for (const worker of workers) {
      worker.terminate();
    }

    // Shutdown native network client
    if (nativeNetClient) {
      nativeNetClient.shutdown();
    }

    if (process.stdin.isTTY && (process.stdin as any).setRawMode) {
      (process.stdin as any).setRawMode(false);
    }
    process.stdin.pause();

    process.exit(code);
  };

  // Handle Ctrl+C via signal as a fallback
  process.on('SIGINT', () => {
    shutdown(0);
  });

  // Configure stdin → VM UART input
  if (process.stdin.isTTY && (process.stdin as any).setRawMode) {
    (process.stdin as any).setRawMode(true);
  }
  process.stdin.resume();

  process.stdin.on('data', (chunk) => {
    // In raw mode `chunk` is typically a Buffer; iterate its bytes.
    for (const byte of chunk as any as Uint8Array) {
      // Ctrl+C (ETX) – terminate the VM and exit
      if (byte === 3) {
        shutdown(0);
        return;
      }

      // Map CR to LF as in the React hook
      if (byte === 13) {
        vm.input(10);
      } else if (byte === 127 || byte === 8) {
        // Backspace
        vm.input(8);
      } else {
        vm.input(byte);
      }
    }
  });

  const INSTRUCTIONS_PER_TICK = 100_000;

  const drainOutput = () => {
    // Drain UART output buffer
    // In raw terminal mode, we need \r\n for proper line breaks
    const outChunks: string[] = [];
    let limit = 2000;
    let code = typeof vm.get_output === 'function' ? vm.get_output() : undefined;

    while (code !== undefined && limit-- > 0) {
      const c = Number(code);

      if (c === 8) {
        // Backspace – move cursor back, erase, move back
        outChunks.push('\b \b');
      } else if (c === 10) {
        // LF -> CRLF for raw terminal mode (like native CLI)
        outChunks.push('\r\n');
      } else if (c === 13) {
        // CR - just output CR
        outChunks.push('\r');
      } else {
        // Pass through all other characters including ANSI escape sequences
        outChunks.push(String.fromCharCode(c));
      }

      code = vm.get_output();
    }

    if (outChunks.length) {
      process.stdout.write(outChunks.join(''));
    }
  };

  // Bridge packets between native WebTransport and VM
  const bridgeNetwork = () => {
    if (!nativeNetClient) return;

    // Check connection status
    if (!networkConnected && nativeNetClient.isRegistered()) {
      networkConnected = true;
      const ip = nativeNetClient.assignedIp();

      // Set IP in VM's external network backend
      const ipBytes = nativeNetClient.assignedIpBytes();
      if (ipBytes && typeof vm.set_external_network_ip === 'function') {
        vm.set_external_network_ip(new Uint8Array(ipBytes));
      }
    }

    // Forward packets from native client to VM (RX)
    let packet = nativeNetClient.recv();
    while (packet) {
      if (typeof vm.inject_network_packet === 'function') {
        // Convert Buffer to Uint8Array for WASM
        vm.inject_network_packet(new Uint8Array(packet));
      }
      packet = nativeNetClient.recv();
    }


    // Forward packets from VM to native client (TX)
    if (typeof vm.extract_network_packet === 'function') {
      let txPacket = vm.extract_network_packet();
      while (txPacket) {
        // txPacket is Uint8Array from WASM, convert to Buffer for native addon
        nativeNetClient.send(Buffer.from(txPacket));
        txPacket = vm.extract_network_packet();
      }
    }
  };

  const loop = () => {
    if (!running) return;

    try {
      // Execute a batch of instructions
      for (let i = 0; i < INSTRUCTIONS_PER_TICK; i++) {
        vm.step();
      }

      // Drain output
      drainOutput();

      // Bridge network packets
      bridgeNetwork();

      // Check if VM has halted (e.g., shutdown command)
      if (typeof vm.is_halted === 'function' && vm.is_halted()) {
        // Drain any remaining output
        drainOutput();

        const haltCode = typeof vm.halt_code === 'function' ? vm.halt_code() : 0n;
        if (haltCode === 0x5555n) {
          console.log('\r\n[VM] Clean shutdown (PASS)');
        } else {
          console.log(`\r\n[VM] Shutdown with code: 0x${haltCode.toString(16)}`);
        }
        shutdown(0);
        return;
      }
    } catch (err) {
      console.error('\r\n[VM] Error while executing:', err);
      shutdown(1);
      return;
    }

    // Schedule the next tick; run as fast as the event loop allows.
    setImmediate(loop);
  };

  loop();
}

/**
 * Run the VM with a GUI window (SDL)
 * Displays framebuffer and handles mouse/touch input
 */
async function runVmWithGui(vm: any, nativeNetClient: any | null, workers: Worker[] = []) {
  // Try to load SDL - returns null if not available
  let sdl: any = null;
  try {
    sdl = await import('@kmamal/sdl');
  } catch {
    console.error('[GUI] SDL not available. Install with: npm install @kmamal/sdl');
    console.error('[GUI] Falling back to headless mode.');
    runVmLoop(vm, nativeNetClient, workers);
    return;
  }

  let running = true;
  const width = 1024;
  const height = 768;

  // Create SDL window
  let window: any = null;
  window = sdl.video.createWindow({
    title: 'RISC-V VM',
    width,
    height,
  });
  console.error('[GUI] Window created');

  // Handle mouse events for touch input
  window.on('mouseButtonDown', (event: any) => {
    if (event.button === 1) { // Left button
      const x = event.x;
      const y = event.y;
      if (typeof vm.send_touch_event === 'function') {
        vm.send_touch_event(x, y, true);
      }
    }
  });

  window.on('mouseButtonUp', (event: any) => {
    if (event.button === 1) {
      const x = event.x;
      const y = event.y;
      if (typeof vm.send_touch_event === 'function') {
        vm.send_touch_event(x, y, false);
      }
    }
  });

  window.on('close', () => {
    running = false;
  });

  // Shutdown function
  const shutdown = (code: number) => {
    if (!running) return;
    running = false;

    // Terminate worker threads
    for (const worker of workers) {
      worker.terminate();
    }

    // Shutdown native network client
    if (nativeNetClient) {
      nativeNetClient.shutdown();
    }

    // Destroy window
    if (window) {
      try {
        window.destroy();
      } catch {
        // Window may already be destroyed
      }
    }

    process.exit(code);
  };

  // Handle Ctrl+C
  process.on('SIGINT', () => {
    shutdown(0);
  });

  const INSTRUCTIONS_PER_TICK = 100_000;
  let lastFrameVersion = 0;
  let networkConnected = false;

  const drainOutput = () => {
    const outChunks: string[] = [];
    let limit = 2000;
    let code = typeof vm.get_output === 'function' ? vm.get_output() : undefined;

    while (code !== undefined && limit-- > 0) {
      const c = Number(code);
      if (c === 8) {
        outChunks.push('\b \b');
      } else if (c === 10) {
        outChunks.push('\r\n');
      } else if (c === 13) {
        outChunks.push('\r');
      } else {
        outChunks.push(String.fromCharCode(c));
      }
      code = vm.get_output();
    }

    if (outChunks.length) {
      process.stdout.write(outChunks.join(''));
    }
  };

  const bridgeNetwork = () => {
    if (!nativeNetClient) return;

    // Check connection status and propagate IP assignment
    if (!networkConnected && nativeNetClient.isRegistered && nativeNetClient.isRegistered()) {
      networkConnected = true;
      const ipBytes = nativeNetClient.assignedIpBytes ? nativeNetClient.assignedIpBytes() : null;
      if (ipBytes && typeof vm.set_external_network_ip === 'function') {
        vm.set_external_network_ip(new Uint8Array(ipBytes));
      }
    }

    // Forward packets from native client to VM (RX)
    let packet = nativeNetClient.recv();
    while (packet) {
      if (typeof vm.inject_network_packet === 'function') {
        vm.inject_network_packet(new Uint8Array(packet));
      }
      packet = nativeNetClient.recv();
    }

    // Forward packets from VM to native client (TX)
    if (typeof vm.extract_network_packet === 'function') {
      let txPacket = vm.extract_network_packet();
      while (txPacket) {
        nativeNetClient.send(Buffer.from(txPacket));
        txPacket = vm.extract_network_packet();
      }
    }
  };

  const updateDisplay = () => {
    if (!window) return;

    try {
      // Check frame version in guest memory
      const gpuFrame = typeof vm.get_gpu_frame === 'function' ? vm.get_gpu_frame() : null;
      const frameVersion = typeof vm.get_gpu_frame_version === 'function' ? vm.get_gpu_frame_version() : 0;

      if (gpuFrame && frameVersion !== lastFrameVersion) {
        lastFrameVersion = frameVersion;

        // Convert RGBA to BGRA for SDL
        const pixels = Buffer.from(gpuFrame);
        for (let i = 0; i < pixels.length; i += 4) {
          const r = pixels[i];
          const b = pixels[i + 2];
          pixels[i] = b;
          pixels[i + 2] = r;
        }

        window.render(width, height, width * 4, 'bgra32', pixels);
      }
    } catch (e: any) {
      if (e.message?.includes('destroyed')) {
        running = false;
      }
    }
  };

  const loop = () => {
    if (!running) return;

    try {
      for (let i = 0; i < INSTRUCTIONS_PER_TICK; i++) {
        vm.step();
      }

      drainOutput();
      bridgeNetwork();
      updateDisplay();

      if (typeof vm.is_halted === 'function' && vm.is_halted()) {
        drainOutput();
        console.log('\r\n[VM] Halted');
        shutdown(0);
        return;
      }
    } catch (err) {
      console.error('\r\n[VM] Error:', err);
      shutdown(1);
      return;
    }

    setImmediate(loop);
  };

  loop();
}

/**
 * Print banner matching native VM output
 */
function printBanner(sdcardPath: string, numHarts: number, netWebtransport?: string, enableGpu = false) {
  const sdcardName = path.basename(sdcardPath);

  console.log();
  console.log('╔══════════════════════════════════════════════════════════════╗');
  if (enableGpu) {
    console.log('║        RISC-V Emulator with OpenSBI (GUI)                   ║');
  } else {
    console.log('║            RISC-V Emulator with OpenSBI                      ║');
  }
  console.log('╠══════════════════════════════════════════════════════════════╣');
  console.log(`║  SD Card: ${sdcardName.padEnd(50)} ║`);
  console.log(`║  Harts:   ${String(numHarts).padEnd(50)} ║`);
  if (netWebtransport) {
    console.log(`║  Network: ${netWebtransport.padEnd(50)} ║`);
  }
  console.log('╚══════════════════════════════════════════════════════════════╝');
  console.log();
}

const argv = (yargs(hideBin(process.argv)) as any)
  .usage('Usage: $0 [options]')
  .option('sdcard', {
    alias: 's',
    type: 'string',
    describe: 'Path to SD card image (contains kernel + filesystem)',
    demandOption: true,
  })
  .option('harts', {
    alias: 'n',
    type: 'number',
    describe: 'Number of harts (omit or 0 = auto-detect as CPU/2, >= 1 = explicit count)',
  })
  .option('net-webtransport', {
    type: 'string',
    describe: 'WebTransport relay URL for networking (e.g., https://127.0.0.1:4433)',
  })
  .option('cert-hash', {
    type: 'string',
    describe: 'Certificate hash for WebTransport (for self-signed certs)',
  })
  .option('debug', {
    type: 'boolean',
    describe: 'Enable debug output',
    default: false,
  })
  .option('enable-gpu', {
    type: 'boolean',
    describe: 'Enable GPU display (opens a window)',
    default: false,
  })
  .help()
  .version()
  .parseSync();

(async () => {
  const sdcardPath = argv.sdcard as string;
  const hartsArg = argv.harts as number | undefined;
  const netWebtransport = argv['net-webtransport'] as string | undefined;
  const certHash = argv['cert-hash'] as string | undefined;
  const debug = argv.debug as boolean;
  const enableGpu = argv['enable-gpu'] as boolean;

  // Hart count logic:
  // - undefined or 0: auto-detect (cpu/2)
  // - >= 1: use the user-specified value (capped at available CPUs)
  let numHarts: number;
  if (hartsArg === undefined || hartsArg === 0) {
    numHarts = detectHartCount();
  } else if (hartsArg >= 1) {
    // Respect user-specified count, but cap at available CPUs for sanity
    const maxHarts = os.cpus().length;
    numHarts = Math.min(hartsArg, maxHarts);
    if (hartsArg > maxHarts) {
      console.error(`[CLI] Warning: requested ${hartsArg} harts but only ${maxHarts} CPUs available, using ${numHarts}`);
    }
  } else {
    // Invalid value (negative), fall back to auto-detect
    console.error(`[CLI] Warning: invalid harts value ${hartsArg}, using auto-detect`);
    numHarts = detectHartCount();
  }

  // Print banner
  printBanner(sdcardPath, numHarts, netWebtransport, enableGpu);

  try {
    const { vm, nativeNetClient, workers } = await createVm(sdcardPath, {
      harts: numHarts,
      netWebtransport,
      certHash,
      debug,
      enableGpu,
    });

    // Run with GUI or headless based on flag
    if (enableGpu) {
      await runVmWithGui(vm, nativeNetClient, workers);
    } else {
      runVmLoop(vm, nativeNetClient, workers);
    }
  } catch (err) {
    console.error('[CLI] Failed to start VM:', err);
    process.exit(1);
  }
})();

