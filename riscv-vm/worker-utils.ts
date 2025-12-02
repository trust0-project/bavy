/**
 * Worker utility types and functions.
 * 
 * This module contains only types and side-effect-free utility functions
 * that can be safely imported in Node.js or browser environments.
 * 
 * The actual worker entry point is in worker.ts (browser-only).
 */

// ============================================================================
// Message Types
// ============================================================================

/** Message sent from main thread to initialize the worker */
export interface WorkerInitMessage {
  hartId: number;
  /** SharedArrayBuffer containing control + CLINT + DRAM */
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
  stepCount?: number;
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
// Shared Memory Layout (must match shared_mem.rs)
// ============================================================================

/** Control region offsets (i32 indices) */
const CTRL_HALT_REQUESTED = 0;
const CTRL_HALTED = 1;

// ============================================================================
// Utility Functions (side-effect-free, Node.js compatible)
// ============================================================================

/**
 * Check if halt has been requested in the shared control region.
 * This can be used for JS-side polling if needed.
 */
export function isHaltRequested(sharedMem: SharedArrayBuffer): boolean {
  const view = new Int32Array(sharedMem);
  return Atomics.load(view, CTRL_HALT_REQUESTED) !== 0;
}

/**
 * Request halt by setting the flag in shared memory.
 * This can be called from any thread.
 */
export function requestHalt(sharedMem: SharedArrayBuffer): void {
  const view = new Int32Array(sharedMem);
  Atomics.store(view, CTRL_HALT_REQUESTED, 1);
  Atomics.notify(view, CTRL_HALT_REQUESTED);
}

/**
 * Check if VM has halted.
 */
export function isHalted(sharedMem: SharedArrayBuffer): boolean {
  const view = new Int32Array(sharedMem);
  return Atomics.load(view, CTRL_HALTED) !== 0;
}


