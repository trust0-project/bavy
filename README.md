# Linux Riscv Virtual machine
This project contain a virtual machine work in progress built in rust that is currently compatible with xv6 linux kernel.
Allows you to boot a real operating system in WASM environments, nodejs and browser.

## Instructions

Running tests
```bash
cargo test
```

Running the virtual machine from source
```
cargo run --release -- --kernel xv6/kernel --disk xv6/fs.img
```

## Debugging 
xv6 directory has the kernel.asm assembly language decoded version of the kernel to help debug and implement new features
