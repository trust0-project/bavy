# RISC-V Virtual Machine

A complete RISC-V 64-bit (RV64GC) virtual machine implementation in Rust, capable of running modern operating systems like Linux (xv6) and custom bare-metal kernels. It is designed to run both natively and in the browser via WebAssembly.

## Features

- **Core**: RV64GC instruction set implementation (IMAFDC + Zicsr + Zifencei). Integer, atomic and compressed instructions run in a superblock engine with a devirtualized memory fast path; F/D floating-point executes in the interpreter (NaN boxing, all rounding modes for conversions, fcsr flags). Misaligned scalar accesses to RAM are handled in hardware style (no trap).
- **Memory**: Sv39 Virtual Memory Management Unit (MMU) with TLB.
- **Peripherals**:
  - **UART**: 16550-compatible serial console.
  - **PLIC**: Platform-Level Interrupt Controller.
  - **CLINT**: Core Local Interruptor (Timer).
  - **VirtIO**: Block Device (Disk) and Network Device (Net).
- **Networking**: WebTransport relay via `--net-webtransport` (native and Node CLI). There is no `--net-ws` flag.
- **Boards**: `--machine virt` (default, QEMU-virt, 10 MHz timebase) or `--machine d1` (Allwinner D1, 24 MHz). Same values in native `src/main.rs`, Node `cli.ts`, and JS `createVM({ machine })`.
- **Platform**:
  - **WASM**: Compiles to WebAssembly for browser / Node execution.
  - **Native**: Runs as a CLI application on the host OS.
  - Without `SharedArrayBuffer` (no COOP/COEP), Wasm is always **1 hart**.

## Usage

### CLI (Native)

Boots an SD card image (MBR + FAT32 `KERNEL.BIN` + filesystem partition). There is no `--kernel` or `--disk` flag.

```bash
# Boot an SD card (machine defaults to virt, 10 MHz)
cargo run --release -- --sdcard path/to/sdcard.img

# Hart count (`0` = auto: virt uses host CPUs, d1 stays at 1)
cargo run --release -- --sdcard path/to/sdcard.img --harts 2

# Guest board: virt (QEMU-virt) or d1 (Allwinner, 24 MHz timebase)
cargo run --release -- --sdcard path/to/sdcard.img --machine virt
cargo run --release -- --sdcard path/to/sdcard.img --machine d1

# Networking (WebTransport relay)
cargo run --release -- --sdcard path/to/sdcard.img --net-webtransport https://127.0.0.1:4433
```

The Node CLI (`npx virtual-machine`) takes the same `--sdcard`, `--harts`, `--machine`, and `--net-webtransport` options.

### WebAssembly

The VM exposes a simple API for JavaScript integration. Pass `machine: "virt" | "d1"` to match the guest kernel. Without `SharedArrayBuffer`, SMP is unavailable and the VM runs 1 hart.

```typescript
import { createVM } from "virtual-machine";

const vm = await createVM(kernelBytes, { harts: 2, machine: "virt" });

while (running) {
  vm.step();
}
```

## Architecture

The VM follows a modular design:
- `cpu.rs`: Instruction decoder and execution pipeline.
- `mmu.rs`: Virtual address translation.
- `bus.rs`: Memory mapping and device routing.
- `virtio.rs`: VirtIO device implementations.
- `net.rs`: Network backend abstraction.

## Build

```bash
# Build native CLI
cargo build --release

# Build WASM package
wasm-pack build --target web
```


