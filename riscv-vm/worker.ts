/**
 * Web Worker entry point for RISC-V hart.
 *
 * This worker runs a secondary RISC-V hart sharing memory with the main thread
 * via SharedArrayBuffer.
 *
 * Communication protocol:
 *   Main → Worker: WorkerInitMessage
 *   Worker → Main: WorkerReadyMessage | WorkerHaltedMessage | WorkerErrorMessage
 */

import init, { worker_entry } from "./pkg/riscv_vm.js";

// ============================================================================
// Message Types
// ============================================================================

/** Message sent from main thread to initialize the worker */
export interface WorkerInitMessage {
  hartId: number;
  sharedMem: SharedArrayBuffer;
  entryPc: number;
}

/** Message sent when worker is ready to execute */
export interface WorkerReadyMessage {
  type: "ready";
  hartId: number;
}

/** Message sent when worker has halted */
export interface WorkerHaltedMessage {
  type: "halted";
  hartId: number;
}

/** Message sent when an error occurs */
export interface WorkerErrorMessage {
  type: "error";
  hartId?: number;
  error: string;
}

export type WorkerOutboundMessage =
  | WorkerReadyMessage
  | WorkerHaltedMessage
  | WorkerErrorMessage;

// ============================================================================
// Worker Context
// ============================================================================

// Worker global scope type (avoids needing WebWorker lib which conflicts with DOM)
interface WorkerGlobalScope {
  onmessage: ((event: MessageEvent<WorkerInitMessage>) => void) | null;
  onerror: ((event: ErrorEvent) => void) | null;
  postMessage(message: WorkerOutboundMessage): void;
}

declare const self: WorkerGlobalScope;

let initialized = false;

self.onmessage = async (event: MessageEvent<WorkerInitMessage>) => {
  const { hartId, sharedMem, entryPc } = event.data;

  console.log(`[Worker ${hartId}] Received init message`);

  if (!initialized) {
    try {
      // Initialize WASM module
      await init();
      initialized = true;
      console.log(`[Worker ${hartId}] WASM initialized`);
    } catch (e) {
      console.error(`[Worker ${hartId}] WASM init failed:`, e);
      const msg: WorkerErrorMessage = {
        type: "error",
        hartId,
        error: String(e),
      };
      self.postMessage(msg);
      return;
    }
  }

  // Notify main thread that we're ready
  const readyMsg: WorkerReadyMessage = { type: "ready", hartId };
  self.postMessage(readyMsg);

  try {
    // Convert entryPc (number/float64) to BigInt for u64
    const pc = BigInt(Math.floor(entryPc));
    console.log(`[Worker ${hartId}] Starting execution at PC=0x${pc.toString(16)}`);
    worker_entry(hartId, sharedMem, pc);
  } catch (e) {
    console.error(`[Worker ${hartId}] Execution error:`, e);
    const msg: WorkerErrorMessage = {
      type: "error",
      hartId,
      error: String(e),
    };
    self.postMessage(msg);
    return;
  }

  // Signal completion
  console.log(`[Worker ${hartId}] Halted`);
  const haltedMsg: WorkerHaltedMessage = { type: "halted", hartId };
  self.postMessage(haltedMsg);
};

self.onerror = (e: ErrorEvent) => {
  console.error("[Worker] Uncaught error:", e);
  const msg: WorkerErrorMessage = {
    type: "error",
    error: e.message || String(e),
  };
  self.postMessage(msg);
};
