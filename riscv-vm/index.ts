import wasmBuffer from "./pkg/riscv_vm_bg.wasm";

let loaded: typeof import("./pkg/riscv_vm") | undefined;

export async function WasmInternal() {
  if (!loaded) {
    const module = await import("./pkg/riscv_vm");
    const wasmInstance = module.initSync(wasmBuffer);
    await module.default(wasmInstance);
    loaded = module;
  }
  return loaded;
}

export { NetworkStatus, WasmVm } from "./pkg/riscv_vm";

// Re-export worker message types for consumers (from side-effect-free module)
export type {
  WorkerInitMessage,
  WorkerReadyMessage,
  WorkerHaltedMessage,
  WorkerErrorMessage,
  WorkerOutboundMessage,
} from "./worker-utils";

// Re-export worker utilities (from side-effect-free module)
export { isHaltRequested, requestHalt, isHalted } from "./worker-utils";

// ============================================================================
// Multi-Hart Worker Support
// ============================================================================

export interface VmOptions {
  /** Number of harts (auto-detected if not specified) */
  harts?: number;
  /** Path to worker script (default: '/worker.js') */
  workerScript?: string;
}

/**
 * Create a VM instance with optional SMP support.
 *
 * If SharedArrayBuffer is available (requires COOP/COEP headers), the VM
 * will run in true parallel mode with Web Workers for secondary harts.
 *
 * NOTE: In WASM, multi-hart mode is significantly slower due to
 * SharedArrayBuffer/Atomics overhead (see tasks/improvements.md).
 * Default is auto-detect (cpu/2) unless explicitly specified.
 *
 * @param kernelData - ELF kernel binary
 * @param options - VM configuration options
 * @param options.harts - Number of harts: undefined/0 = auto-detect (cpu/2), >= 1 = explicit count
 * @returns WasmVm instance
 */
export async function createVM(
  kernelData: Uint8Array,
  options: VmOptions = {}
): Promise<import("./pkg/riscv_vm").WasmVm> {
  const module = await WasmInternal();

  // Hart count logic:
  // - undefined or 0: auto-detect (cpu/2) via Rust default constructor
  // - >= 1: use the specified value via new_with_harts
  const harts = options.harts;

  // Create VM with specified hart count
  // new_with_harts(harts) for explicit count, default constructor for auto-detect
  const vm = (harts !== undefined && harts >= 1)
    ? module.WasmVm.new_with_harts(kernelData, harts)
    : new module.WasmVm(kernelData);

  // Start workers if in SMP mode
  const workerScript = options.workerScript || "/worker.js";
  if (vm.is_smp()) {
    try {
      vm.start_workers(workerScript);
      console.log(`[VM] Started workers for ${vm.num_harts()} harts`);
    } catch (e) {
      console.warn("[VM] Failed to start workers, falling back to single-threaded:", e);
    }
  }

  console.log(`[VM] Created VM instance (SMP: ${vm.is_smp()}, harts: ${vm.num_harts()})`);

  return vm;
}

// ============================================================================
// SD Card Boot Support
// ============================================================================

interface SDBootInfo {
  kernelData: Uint8Array;
  sdcardData: Uint8Array;
  fsPartitionStart: number;
}

/**
 * Parse an SD card image to extract kernel from FAT32 boot partition.
 * 
 * SD Card layout:
 * - MBR with partition table
 * - Partition 1 (FAT32): kernel.bin
 * - Partition 2: SFS filesystem
 */
export function parseSDCard(sdcard: Uint8Array): SDBootInfo {
  if (sdcard.length < 512) {
    throw new Error('SD card image too small');
  }

  // Check MBR signature
  if (sdcard[510] !== 0x55 || sdcard[511] !== 0xAA) {
    throw new Error('Invalid MBR signature');
  }

  // Parse partition table
  let bootPartStart = 0;
  let fsPartStart = 0;

  for (let i = 0; i < 4; i++) {
    const offset = 446 + i * 16;
    const partType = sdcard[offset + 4];
    const startLba = sdcard[offset + 8] | (sdcard[offset + 9] << 8) |
      (sdcard[offset + 10] << 16) | (sdcard[offset + 11] << 24);

    if ((partType === 0x0B || partType === 0x0C) && bootPartStart === 0) {
      bootPartStart = startLba;
    } else if (partType !== 0 && fsPartStart === 0 && startLba !== bootPartStart) {
      fsPartStart = startLba;
    }
  }

  if (bootPartStart === 0) {
    throw new Error('No FAT32 boot partition found');
  }

  // Parse FAT32 boot sector
  const bootOffset = bootPartStart * 512;
  const bytesPerSector = sdcard[bootOffset + 11] | (sdcard[bootOffset + 12] << 8);
  const sectorsPerCluster = sdcard[bootOffset + 13];
  const reservedSectors = sdcard[bootOffset + 14] | (sdcard[bootOffset + 15] << 8);
  const numFats = sdcard[bootOffset + 16];
  const sectorsPerFat = sdcard[bootOffset + 36] | (sdcard[bootOffset + 37] << 8) |
    (sdcard[bootOffset + 38] << 16) | (sdcard[bootOffset + 39] << 24);
  const rootCluster = sdcard[bootOffset + 44] | (sdcard[bootOffset + 45] << 8) |
    (sdcard[bootOffset + 46] << 16) | (sdcard[bootOffset + 47] << 24);

  // Find kernel.bin in root directory
  const dataStartSector = reservedSectors + (numFats * sectorsPerFat);
  const rootDirSector = dataStartSector + (rootCluster - 2) * sectorsPerCluster;
  const rootDirOffset = bootOffset + (rootDirSector * bytesPerSector);
  const clusterBytes = sectorsPerCluster * bytesPerSector;

  let kernelData: Uint8Array | null = null;

  for (let i = 0; i < clusterBytes && (rootDirOffset + i + 32) <= sdcard.length; i += 32) {
    const entryOffset = rootDirOffset + i;
    if (sdcard[entryOffset] === 0x00) break;
    if (sdcard[entryOffset] === 0xE5 || sdcard[entryOffset + 11] === 0x0F) continue;

    const name = String.fromCharCode(...sdcard.slice(entryOffset, entryOffset + 11));
    if (name === 'KERNEL  BIN') {
      const clusterHigh = sdcard[entryOffset + 20] | (sdcard[entryOffset + 21] << 8);
      const clusterLow = sdcard[entryOffset + 26] | (sdcard[entryOffset + 27] << 8);
      const fileSize = sdcard[entryOffset + 28] | (sdcard[entryOffset + 29] << 8) |
        (sdcard[entryOffset + 30] << 16) | (sdcard[entryOffset + 31] << 24);
      const fileCluster = (clusterHigh << 16) | clusterLow;
      const fileSector = dataStartSector + (fileCluster - 2) * sectorsPerCluster;
      const fileOffset = bootOffset + (fileSector * bytesPerSector);

      if (fileOffset + fileSize <= sdcard.length) {
        kernelData = sdcard.slice(fileOffset, fileOffset + fileSize);
      }
      break;
    }
  }

  if (!kernelData) {
    throw new Error('kernel.bin not found on boot partition');
  }

  return { kernelData, sdcardData: sdcard, fsPartitionStart: fsPartStart };
}

/**
 * Fetch an SD card image from a URL for browser use.
 * 
 * @param url - URL to fetch SD card image from
 * @returns Parsed boot info with kernel and disk data
 */
export async function loadSDCardFromUrl(url: string): Promise<SDBootInfo> {
  console.log(`[SDK] Fetching SD card from ${url}...`);
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to fetch SD card: ${response.statusText}`);
  }
  const arrayBuffer = await response.arrayBuffer();
  const sdcardData = new Uint8Array(arrayBuffer);
  console.log(`[SDK] Loaded ${sdcardData.length} bytes`);
  return parseSDCard(sdcardData);
}

/**
 * Create a VM from an SD card image (fetched from URL).
 * 
 * This is the recommended way to boot the VM in browser.
 * 
 * @param sdcardUrl - URL to SD card image
 * @param options - VM configuration options
 * @returns WasmVm instance ready to run
 */
export async function createVMFromSDCard(
  sdcardUrl: string,
  options: VmOptions = {}
): Promise<import("./pkg/riscv_vm").WasmVm> {
  const bootInfo = await loadSDCardFromUrl(sdcardUrl);
  const vm = await createVM(bootInfo.kernelData, options);

  // Load entire SD card as block device for filesystem access
  if (typeof vm.load_disk === 'function') {
    vm.load_disk(bootInfo.sdcardData);
    console.log(`[SDK] Mounted SD card (fs partition at sector ${bootInfo.fsPartitionStart})`);
  }

  return vm;
}

/**
 * Run the VM with an output callback for UART data.
 *
 * This function manages the main execution loop, stepping hart 0 on the
 * main thread. Secondary harts (if any) run in Web Workers.
 *
 * @param vm - WasmVm instance
 * @param onOutput - Callback for each character output
 * @param options - Run options
 * @returns Stop function to halt execution
 */
export function runVM(
  vm: import("./pkg/riscv_vm").WasmVm,
  onOutput: (char: string) => void,
  options: { stepsPerFrame?: number } = {}
): () => void {
  const stepsPerFrame = options.stepsPerFrame || 10000;
  let running = true;

  const loop = () => {
    if (!running) return;

    // Step primary hart (I/O coordination)
    for (let i = 0; i < stepsPerFrame; i++) {
      if (!vm.step()) {
        console.log("[VM] Halted");
        running = false;
        return;
      }
    }

    // Collect output
    let byte: number | undefined;
    while ((byte = vm.get_output()) !== undefined) {
      onOutput(String.fromCharCode(byte));
    }

    // Schedule next batch
    requestAnimationFrame(loop);
  };

  loop();

  // Return stop function
  return () => {
    running = false;
    vm.terminate_workers();
  };
}

// ============================================================================
// SharedArrayBuffer Support Detection
// ============================================================================

export interface SharedMemorySupport {
  supported: boolean;
  crossOriginIsolated: boolean;
  message: string;
}

/**
 * Check if SharedArrayBuffer is available for multi-threaded execution.
 *
 * SharedArrayBuffer requires Cross-Origin Isolation (COOP/COEP headers).
 * If not available, the VM will run in single-threaded mode.
 */
export function checkSharedMemorySupport(): SharedMemorySupport {
  const crossOriginIsolated = isCrossOriginIsolated();

  if (typeof SharedArrayBuffer === "undefined") {
    return {
      supported: false,
      crossOriginIsolated,
      message: "SharedArrayBuffer not defined. Browser may be too old.",
    };
  }

  if (!crossOriginIsolated) {
    return {
      supported: false,
      crossOriginIsolated,
      message:
        "Not cross-origin isolated. Add headers:\n" +
        "  Cross-Origin-Opener-Policy: same-origin\n" +
        "  Cross-Origin-Embedder-Policy: require-corp",
    };
  }

  // Try to create a SharedArrayBuffer
  try {
    new SharedArrayBuffer(8);
    return {
      supported: true,
      crossOriginIsolated,
      message: "SharedArrayBuffer available for SMP execution",
    };
  } catch (e) {
    return {
      supported: false,
      crossOriginIsolated,
      message: `SharedArrayBuffer blocked: ${e}`,
    };
  }
}

/**
 * Check if the page is cross-origin isolated (required for SharedArrayBuffer).
 */
export function isCrossOriginIsolated(): boolean {
  return typeof crossOriginIsolated !== "undefined" && crossOriginIsolated;
}

// ============================================================================
// COOP/COEP Headers Reference
// ============================================================================

/**
 * Headers required for SharedArrayBuffer support.
 *
 * For Vite dev server, add to vite.config.ts:
 * ```ts
 * server: {
 *   headers: {
 *     "Cross-Origin-Opener-Policy": "same-origin",
 *     "Cross-Origin-Embedder-Policy": "require-corp",
 *   },
 * },
 * ```
 *
 * For production, configure your web server to add these headers.
 */
export const REQUIRED_HEADERS = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
} as const;

// ============================================================================
// Worker Management Utilities
// ============================================================================

/**
 * Manually create and manage workers for advanced use cases.
 *
 * Most users should use createVM() which handles workers automatically.
 */
export interface WorkerManager {
  /** Start a worker for a specific hart */
  startWorker(
    hartId: number,
    sharedMem: SharedArrayBuffer,
    entryPc: number,
    workerScript?: string
  ): Worker;
  /** Terminate all workers */
  terminateAll(): void;
  /** Get number of active workers */
  count(): number;
}

/**
 * Create a worker manager for manual worker control.
 */
export function createWorkerManager(): WorkerManager {
  const workers: Worker[] = [];

  return {
    startWorker(
      hartId: number,
      sharedMem: SharedArrayBuffer,
      entryPc: number,
      workerScript = "/worker.js"
    ): Worker {
      const worker = new Worker(workerScript, { type: "module" });

      worker.onmessage = (event) => {
        const { type, hartId: id, error } = event.data;
        switch (type) {
          case "ready":
            console.log(`[WorkerManager] Hart ${id} ready`);
            break;
          case "halted":
            console.log(`[WorkerManager] Hart ${id} halted`);
            break;
          case "error":
            console.error(`[WorkerManager] Hart ${id} error:`, error);
            break;
        }
      };

      worker.onerror = (e) => {
        console.error(`[WorkerManager] Worker error:`, e);
      };

      // Send init message
      worker.postMessage({
        hartId,
        sharedMem,
        entryPc,
      });

      workers.push(worker);
      return worker;
    },

    terminateAll(): void {
      for (const worker of workers) {
        worker.terminate();
      }
      workers.length = 0;
    },

    count(): number {
      return workers.length;
    },
  };
}
