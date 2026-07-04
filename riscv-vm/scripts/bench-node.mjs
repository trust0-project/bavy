#!/usr/bin/env node
// Node benchmark driver for the WASM build of riscv-vm.
//
// Runs the synthetic guest workloads (see src/bench/) on hart 0 and reports
// MIPS based on retired guest instructions (WasmVm.instret), which counts
// real instructions even when the superblock engine retires many per step.
//
// Usage: node scripts/bench-node.mjs [seconds-per-workload] [workload]
// Requires the package to be built first (yarn build).

import { WasmInternal } from '../build/index.mjs';

const seconds = parseFloat(process.argv[2] ?? '2');
const only = process.argv[3];
const workloads = only ? [only] : ['nop', 'prime', 'memcpy', 'ecall'];

const wasm = await WasmInternal();
const { WasmVm, bench_workload } = wasm;

if (typeof bench_workload !== 'function') {
  console.error('bench_workload export missing - rebuild the package (yarn build)');
  process.exit(1);
}

console.log(`riscv-vm node benchmark | ${seconds.toFixed(1)}s per workload`);
console.log('workload      instructions        MIPS');
console.log('--------------------------------------');

for (const name of workloads) {
  const binary = bench_workload(name);
  const vm = WasmVm.new_with_harts(binary, 1);
  // Workloads are bare M-mode programs; undo the S-mode kernel boot setup.
  vm.reset_machine_mode();

  const start = performance.now();
  const deadlineMs = start + seconds * 1000;
  // Large step batches amortize the JS<->WASM boundary crossing.
  const run = typeof vm.run_batch === 'function'
    ? (n) => vm.run_batch(n)
    : (n) => vm.step_n(n);
  while (performance.now() < deadlineMs) {
    run(500_000);
  }
  const elapsed = (performance.now() - start) / 1000;
  const instret = vm.instret();
  const mips = instret / elapsed / 1e6;

  console.log(
    `${name.padEnd(10)} ${String(Math.round(instret)).padStart(15)} ${mips.toFixed(2).padStart(11)}`
  );

  vm.free();
}
