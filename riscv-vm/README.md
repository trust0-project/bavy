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
- **Networking**:
  - Native TAP interface support (Linux).
  - WebSocket backend for browser/cross-platform networking.
  - WebTransport backend for P2P connectivity.
- **Platform**:
  - **WASM**: Compiles to WebAssembly for browser execution.
  - **Native**: Runs as a CLI application on Host OS.

## Usage

### CLI (Native)

Run the emulator from the command line:

```bash
# Run a kernel image
cargo run --release -- --kernel path/to/kernel

# Run with networking (WebSocket backend)
cargo run --release -- --kernel path/to/kernel --net-ws ws://localhost:8765

# Run with block device
cargo run --release -- --kernel path/to/kernel --disk path/to/fs.img
```

### WebAssembly

The VM exposes a simple API for JavaScript integration:

```typescript
import { WasmVm } from "virtual-machine";

// Initialize VM with kernel binary
const vm = new WasmVm(kernelBytes);

// Connect networking
vm.connect_network("ws://localhost:8765");

// Step execution
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


