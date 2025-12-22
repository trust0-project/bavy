pub mod block;
pub mod device;
pub mod input;
pub mod net;
pub mod p9;
#[cfg(target_arch = "wasm32")]
pub mod p9_wasm;
pub mod rng;

// Re-export common types for convenience
pub use block::VirtioBlock;
pub use device::VirtioDevice;
pub use input::VirtioInput;
pub use net::VirtioNet;
pub use p9::VirtioP9;
#[cfg(target_arch = "wasm32")]
pub use p9_wasm::VirtioP9Wasm;
pub use rng::VirtioRng;

