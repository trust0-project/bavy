/**
 * Node.js Worker Thread entry point for RISC-V hart.
 *
 * This worker runs a secondary RISC-V hart (1, 2, 3, ...) sharing memory
 * with the main thread via SharedArrayBuffer using Node.js worker_threads.
 *
 * ## Architecture
 *
 * - Workers run secondary harts (1+)
 * - Hart 0 runs on main thread (handles I/O)
 * - Shared via SharedArrayBuffer:
 *   - Control region (halt flags)
 *   - CLINT (timer, IPI)
 *   - DRAM (kernel memory)
 *
 * ## Communication Protocol
 *
 * Main → Worker: workerData (hartId, sharedMem, entryPc)
 * Worker → Main: WorkerReadyMessage | WorkerHaltedMessage | WorkerErrorMessage
 */

import { parentPort, workerData } from 'node:worker_threads';

// Import WASM as embedded buffer (converted to base64 by tsup wasmPlugin)
import wasmBuffer from "./pkg/riscv_vm_bg.wasm";
import { initSync, WorkerState } from "./pkg/riscv_vm.js";

// WorkerStepResult enum values (must match worker.rs)
const WorkerStepResult = {
  Continue: 0,
  Halted: 1,
  Shutdown: 2,
  Error: 3,
  Wfi: 4,  // WFI executed - yield to save CPU
} as const;

// ============================================================================
// Message Types
// ============================================================================

interface WorkerData {
  hartId: number;
  sharedMem: SharedArrayBuffer;
  entryPc: number;
}

interface WorkerReadyMessage {
  type: "ready";
  hartId: number;
}

interface WorkerHaltedMessage {
  type: "halted";
  hartId: number;
  stepCount?: number;
}

interface WorkerErrorMessage {
  type: "error";
  hartId?: number;
  error: string;
}

type WorkerOutboundMessage =
  | WorkerReadyMessage
  | WorkerHaltedMessage
  | WorkerErrorMessage;

// ============================================================================
// Shared Memory Layout (must match shared_mem.rs)
// ============================================================================

const CTRL_HALT_REQUESTED = 0;

// ============================================================================
// Worker Context
// ============================================================================

let controlView: Int32Array | null = null;
let workerState: any = null;

// Tuning parameters for execution
const BATCH_SIZE = 100_000;
const BATCHES_PER_YIELD = 10;

function postMessage(msg: WorkerOutboundMessage): void {
  parentPort?.postMessage(msg);
}

/**
 * Log a message to main thread via postMessage (more reliable than console.log).
 * This ensures logs are visible even when workers are blocking.
 */
function logToMain(hartId: number, message: string): void {
  parentPort?.postMessage({ type: "log", hartId, message });
}

/**
 * Run loop using optimized blocking execution with periodic yields.
 */
function runLoop(hartId: number): void {
  if (!workerState || !controlView) {
    console.error('[Worker] runLoop called without workerState or controlView');
    return;
  }

  let shouldContinue = true;
  let batchCount = 0;
  let totalBatches = 0; // Persistent counter that doesn't reset

  while (shouldContinue) {
    // Log before first batch to see if we're even entering the loop
    if (totalBatches === 0) {
      logToMain(hartId, "About to call first step_batch...");
    }

    const result = workerState.step_batch(BATCH_SIZE);

    // Log after first batch returns
    if (totalBatches === 0) {
      logToMain(hartId, `First step_batch returned: ${result}, steps=${workerState.step_count()}`);
    }

    switch (result) {
      case WorkerStepResult.Continue:
        batchCount++;
        totalBatches++;


        // Yield periodically to allow halt signals to be processed
        if (batchCount >= BATCHES_PER_YIELD) {
          batchCount = 0;

          // Use Atomics.wait with 0ms timeout for efficient yielding
          try {
            Atomics.wait(controlView, CTRL_HALT_REQUESTED, 0, 0);
          } catch {
            // Fall back gracefully
          }
        }
        break;

      case WorkerStepResult.Halted:
        postMessage({ type: "halted", hartId, stepCount: Number(workerState.step_count()) });
        cleanup();
        shouldContinue = false;
        break;

      case WorkerStepResult.Shutdown:
        postMessage({ type: "halted", hartId, stepCount: Number(workerState.step_count()) });
        cleanup();
        shouldContinue = false;
        break;

      case WorkerStepResult.Error:
        postMessage({ type: "error", hartId, error: "Execution error" });
        cleanup();
        shouldContinue = false;
        break;

      case WorkerStepResult.Wfi:
        // WFI executed - yield to save CPU when idle
        // The Rust code already slept via Atomics.wait for up to 100ms,
        // so we add a smaller additional yield here
        try {
          Atomics.wait(controlView, CTRL_HALT_REQUESTED, 0, 50);
        } catch {
          // Fall back gracefully
        }
        break;
    }
  }
}

function cleanup(): void {
  workerState = null;
  controlView = null;
}

async function main(): Promise<void> {
  const { hartId, sharedMem, entryPc } = workerData as WorkerData;

  logToMain(hartId, "Starting with bundled WASM");

  try {
    // Stagger WASM initialization to prevent concurrent compilation
    // from overwhelming the host. Each hart waits (hartId * 10ms).
    const staggerDelay = hartId * 10;
    if (staggerDelay > 0) {
      await new Promise(resolve => setTimeout(resolve, staggerDelay));
    }

    // Initialize WASM with the bundled buffer
    initSync(wasmBuffer);

    logToMain(hartId, "WASM initialized");

    // Verify SharedArrayBuffer
    if (!(sharedMem instanceof SharedArrayBuffer)) {
      throw new Error("sharedMem must be a SharedArrayBuffer");
    }

    // Notify main thread that we're ready
    postMessage({ type: "ready", hartId });

    // Convert entryPc to BigInt for u64
    const pc = BigInt(Math.floor(entryPc));
    logToMain(hartId, `Starting execution at PC=0x${pc.toString(16)}`);

    // Set up control view for Atomics
    controlView = new Int32Array(sharedMem);

    logToMain(hartId, "Creating WorkerState...");

    // Create worker state
    workerState = new WorkerState(hartId, sharedMem, pc);

    // Log a0 register value to verify hart_id is correctly passed to kernel
    const a0 = workerState.get_a0();
    const msipPending = workerState.is_msip_pending();
    const timerPending = workerState.is_timer_pending();
    logToMain(hartId, `WorkerState created, a0=${a0}, msip=${msipPending}, timer=${timerPending}`);

    // Start the run loop
    runLoop(hartId);
  } catch (e) {
    logToMain(hartId, `Error: ${e}`);
    postMessage({
      type: "error",
      hartId,
      error: String(e),
    });
  }
}

// Start the worker
main().catch((e) => {
  console.error('[Worker] Fatal error:', e);
  postMessage({
    type: "error",
    error: String(e),
  });
});

