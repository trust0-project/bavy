//! Virtual Machine implementations.

#[cfg(not(target_arch = "wasm32"))]
pub mod emulator;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod wasm;













