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

// Re-export worker message types for consumers
export type {
  WorkerInitMessage,
  WorkerReadyMessage,
  WorkerHaltedMessage,
  WorkerErrorMessage,
  WorkerOutboundMessage,
} from "./worker";

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
 * Create a VM instance and optionally start workers for multi-hart execution.
 * 
 * @param kernelData - ELF kernel binary
 * @param options - VM configuration options
 * @returns WasmVm instance
 */
export async function createVM(
  kernelData: Uint8Array,
  options: VmOptions = {}
): Promise<import("./pkg/riscv_vm").WasmVm> {
  const module = await WasmInternal();
  
  const vm = new module.WasmVm(kernelData);
  
  // Note: Worker spawning is handled by WasmVm internally if SharedArrayBuffer
  // is available. This function provides a convenient entry point.
  console.log('[VM] Created VM instance');
  
  return vm;
}

/**
 * Run the VM with an output callback for UART data.
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
        console.log('[VM] Halted');
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
  };
}

// ============================================================================
// SharedArrayBuffer Support Detection
// ============================================================================

export interface SharedMemorySupport {
  supported: boolean;
  message: string;
}

/**
 * Check if SharedArrayBuffer is available for multi-threaded execution.
 * 
 * SharedArrayBuffer requires Cross-Origin Isolation (COOP/COEP headers).
 * If not available, the VM will run in single-threaded mode.
 */
export function checkSharedMemorySupport(): SharedMemorySupport {
  if (typeof SharedArrayBuffer === 'undefined') {
    return {
      supported: false,
      message: 'SharedArrayBuffer not available. Check COOP/COEP headers.'
    };
  }
  
  // Try to create a SharedArrayBuffer
  try {
    new SharedArrayBuffer(1);
    return { supported: true, message: 'SharedArrayBuffer available' };
  } catch (e) {
    return {
      supported: false,
      message: `SharedArrayBuffer blocked: ${e}`
    };
  }
}

/**
 * Check if the page is cross-origin isolated (required for SharedArrayBuffer).
 */
export function isCrossOriginIsolated(): boolean {
  return typeof crossOriginIsolated !== 'undefined' && crossOriginIsolated;
}
