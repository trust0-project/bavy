pub mod block;
pub mod device;
pub mod gpu;
pub mod input;
pub mod net;
pub mod rng;

// Re-export common types for convenience
pub use block::VirtioBlock;
pub use device::VirtioDevice;
pub use gpu::VirtioGpu;
pub use input::VirtioInput;
pub use net::VirtioNet;
pub use rng::VirtioRng;

