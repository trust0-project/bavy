//! Wasm codegen stub.
//!
//! Browser and Node share the same MicroOp IR (`crate::engine::microop`) as
//! the native superblock engine. A future `wasm-encoder` backend should lower
//! that IR to a wasm module instantiated on the worker. Do not maintain a third
//! interpreter.

/// No wasm-encoder backend is wired yet.
pub fn backend_available() -> bool {
    false
}
