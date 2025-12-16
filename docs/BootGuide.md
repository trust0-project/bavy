# VM Boot Guide

## Quick Start

```bash
# 1. Build SD card image
cd havy_os
./build_sdcard.sh

# 2. Run VM
cargo run -p riscv-vm --release -- --sdcard target/riscv64gc-unknown-none-elf/release/sdcard.img
```

## SD Card Layout

| Region | Content |
|--------|---------|
| Sector 0 | MBR partition table |
| Part 1 (FAT32) | kernel.bin |
| Part 2 (raw) | SFS filesystem |

## Platform Commands

```bash
# Native
cargo run -p riscv-vm -- --sdcard sdcard.img

# Node.js
node build/cli.js --sdcard sdcard.img

# Browser (JavaScript)
import { createVMFromSDCard, runVM } from 'riscv-vm';

const vm = await createVMFromSDCard('/sdcard.img');
runVM(vm, (char) => console.log(char));
```

## Boot Sequence

1. VM parses MBR → finds FAT32 partition
2. Reads `kernel.bin` from FAT32
3. Loads kernel to 0x8020_0000
4. OpenSBI (built-in) starts kernel in S-mode
5. Kernel mounts Part 2 as block device

## Real Hardware (Lichee RV 86)

```bash
# Write to physical SD card
sudo dd if=sdcard.img of=/dev/sdX bs=1M status=progress

# In U-Boot:
load mmc 0:1 0x40200000 kernel.bin
go 0x40200000
```
