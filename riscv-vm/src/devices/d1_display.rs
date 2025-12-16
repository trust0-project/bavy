//! Emulated D1 Display Engine
//!
//! Emulates the Allwinner D1 display pipeline (DE2 + TCON + DSI) for the VM.
//! On browser (WASM), connects to WebGPU for rendering.
//! On Node.js/native, framebuffer is stored but not displayed.
//!
//! This allows the kernel to use the same D1 display driver on both
//! real hardware and the emulator.

use std::sync::{Arc, RwLock};

#[cfg(target_arch = "wasm32")]
use send_wrapper::SendWrapper;

// Register base addresses (matching D1)
pub const D1_DE_BASE: u64 = 0x0510_0000;
pub const D1_DE_SIZE: u64 = 0x10000;
pub const D1_MIPI_DSI_BASE: u64 = 0x0545_0000;
pub const D1_MIPI_DSI_SIZE: u64 = 0x1000;
pub const D1_DPHY_BASE: u64 = 0x0545_1000;
pub const D1_DPHY_SIZE: u64 = 0x1000;
pub const D1_TCON_LCD0: u64 = 0x0546_1000;
pub const D1_TCON_SIZE: u64 = 0x1000;

// DE2 register offsets
const DE_GLB_CTL: u64 = 0x0000;
const DE_GLB_STS: u64 = 0x0004;
const DE_GLB_DBUFF: u64 = 0x0008;
const DE_GLB_SIZE: u64 = 0x000C;

// UI layer registers (relative to UI base at 0x3000)
const UI_ATTR: u64 = 0x00;
const UI_SIZE: u64 = 0x04;
const UI_COORD: u64 = 0x08;
const UI_PITCH: u64 = 0x0C;
const UI_TOP_LADDR: u64 = 0x10;

// Display parameters - 1024x768 resolution for larger display
const DISPLAY_WIDTH: u32 = 1024;
const DISPLAY_HEIGHT: u32 = 768;
const FRAMEBUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT * 4) as usize;

/// Emulated D1 Display Engine
pub struct D1DisplayEmulated {
    // Framebuffer memory (shared with host for WebGPU access)
    framebuffer: Arc<RwLock<Vec<u32>>>,
    
    // DE2 registers
    glb_ctl: u32,
    glb_size: u32,
    
    // UI layer registers
    ui_attr: u32,
    ui_size: u32,
    ui_coord: u32,
    ui_pitch: u32,
    ui_addr: u32,
    
    // TCON registers
    tcon_gctl: u32,
    tcon_ctl: u32,
    
    // Display state
    width: u32,
    height: u32,
    enabled: bool,
    
    // Callback for framebuffer updates (WebGPU integration)
    // Wrapped in SendWrapper since WASM is single-threaded but Rust's type system
    // requires Send+Sync for RwLock usage in SystemBus
    #[cfg(target_arch = "wasm32")]
    update_callback: Option<SendWrapper<js_sys::Function>>,
}

impl D1DisplayEmulated {
    pub fn new() -> Self {
        let fb_size = (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize;
        Self {
            framebuffer: Arc::new(RwLock::new(vec![0xFF000000; fb_size])),
            glb_ctl: 0,
            glb_size: ((DISPLAY_HEIGHT - 1) << 16) | (DISPLAY_WIDTH - 1),
            ui_attr: 0,
            ui_size: ((DISPLAY_HEIGHT - 1) << 16) | (DISPLAY_WIDTH - 1),
            ui_coord: 0,
            ui_pitch: DISPLAY_WIDTH * 4,
            ui_addr: 0,
            tcon_gctl: 0,
            tcon_ctl: 0,
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            enabled: false,
            #[cfg(target_arch = "wasm32")]
            update_callback: None,
        }
    }

    /// Get shared framebuffer for WebGPU access
    pub fn framebuffer(&self) -> Arc<RwLock<Vec<u32>>> {
        self.framebuffer.clone()
    }

    /// Get framebuffer as bytes (for WebGPU texture upload)
    pub fn framebuffer_bytes(&self) -> Vec<u8> {
        let fb = self.framebuffer.read().unwrap();
        let mut bytes = Vec::with_capacity(fb.len() * 4);
        for &pixel in fb.iter() {
            // Convert ARGB to RGBA for WebGPU
            let a = (pixel >> 24) & 0xFF;
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            bytes.push(r as u8);
            bytes.push(g as u8);
            bytes.push(b as u8);
            bytes.push(a as u8);
        }
        bytes
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set callback for framebuffer updates (browser only)
    #[cfg(target_arch = "wasm32")]
    pub fn set_update_callback(&mut self, callback: js_sys::Function) {
        self.update_callback = Some(SendWrapper::new(callback));
    }

    /// Trigger framebuffer update notification
    fn notify_update(&self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(ref callback) = self.update_callback {
            let _ = callback.call0(&wasm_bindgen::JsValue::NULL);
        }
    }

    /// Copy framebuffer data from guest memory
    pub fn update_framebuffer_from_memory(&self, memory: &[u8], addr: u64) {
        if !self.enabled {
            return;
        }

        let src_addr = addr as usize;
        if src_addr + FRAMEBUFFER_SIZE > memory.len() {
            return;
        }

        let mut fb = self.framebuffer.write().unwrap();
        for i in 0..fb.len() {
            let offset = src_addr + i * 4;
            if offset + 4 <= memory.len() {
                let pixel = u32::from_le_bytes([
                    memory[offset],
                    memory[offset + 1],
                    memory[offset + 2],
                    memory[offset + 3],
                ]);
                fb[i] = pixel;
            }
        }

        drop(fb);
        self.notify_update();
    }
}

// MMIO Access Methods (for bus integration)
impl D1DisplayEmulated {
    pub fn mmio_read32(&self, addr: u64) -> u32 {
        // DE2 registers (0x0510_0000 - 0x051F_FFFF)
        if addr >= D1_DE_BASE && addr < D1_DE_BASE + D1_DE_SIZE {
            let offset = addr - D1_DE_BASE;
            
            match offset {
                0x0000 => self.glb_ctl,       // GLB_CTL
                0x000C => self.glb_size,      // GLB_SIZE
                0x3000 => self.ui_attr,       // UI_ATTR
                0x3004 => self.ui_size,       // UI_SIZE
                0x3008 => self.ui_coord,      // UI_COORD
                0x300C => self.ui_pitch,      // UI_PITCH
                0x3010 => self.ui_addr,       // UI_TOP_LADDR
                _ => 0,
            }
        }
        // TCON registers (0x0546_1000 - 0x0546_1FFF)
        else if addr >= D1_TCON_LCD0 && addr < D1_TCON_LCD0 + D1_TCON_SIZE {
            let offset = addr - D1_TCON_LCD0;
            
            match offset {
                0x00 => self.tcon_gctl,
                0x40 => self.tcon_ctl,
                _ => 0,
            }
        }
        // MIPI DSI registers (0x0545_0000 - 0x0545_0FFF) - stub
        else if addr >= D1_MIPI_DSI_BASE && addr < D1_MIPI_DSI_BASE + D1_MIPI_DSI_SIZE {
            // Return 0 for all DSI reads (stub)
            0
        }
        // D-PHY registers (0x0545_1000 - 0x0545_1FFF) - stub
        else if addr >= D1_DPHY_BASE && addr < D1_DPHY_BASE + D1_DPHY_SIZE {
            // Return 0 for all DPHY reads (stub)
            0
        }
        else {
            0
        }
    }

    pub fn mmio_write32(&mut self, addr: u64, value: u32) {
        // DE2 registers
        if addr >= D1_DE_BASE && addr < D1_DE_BASE + D1_DE_SIZE {
            let offset = addr - D1_DE_BASE;
            
            match offset {
                0x0000 => {
                    self.glb_ctl = value;
                    self.enabled = (value & 1) != 0;
                }
                0x0008 => {
                    // Double buffer register update trigger
                    self.notify_update();
                }
                0x000C => self.glb_size = value,
                0x3000 => self.ui_attr = value,
                0x3004 => self.ui_size = value,
                0x3008 => self.ui_coord = value,
                0x300C => self.ui_pitch = value,
                0x3010 => {
                    self.ui_addr = value;
                    // Framebuffer address set - could trigger copy from guest memory
                }
                _ => {}
            }
        }
        // TCON registers
        else if addr >= D1_TCON_LCD0 && addr < D1_TCON_LCD0 + D1_TCON_SIZE {
            let offset = addr - D1_TCON_LCD0;
            
            match offset {
                0x00 => {
                    self.tcon_gctl = value;
                    if (value & (1 << 31)) != 0 {
                        self.enabled = true;
                    }
                }
                0x40 => self.tcon_ctl = value,
                _ => {}
            }
        }
        // MIPI DSI registers - stub (ignore writes)
        else if addr >= D1_MIPI_DSI_BASE && addr < D1_MIPI_DSI_BASE + D1_MIPI_DSI_SIZE {
            // Ignore DSI writes (stub)
        }
        // D-PHY registers - stub (ignore writes)
        else if addr >= D1_DPHY_BASE && addr < D1_DPHY_BASE + D1_DPHY_SIZE {
            // Ignore DPHY writes (stub)
        }
    }

    pub fn mmio_read8(&self, addr: u64) -> u8 {
        let word = self.mmio_read32(addr & !3);
        let shift = (addr & 3) * 8;
        (word >> shift) as u8
    }

    #[allow(unused_variables)]
    pub fn mmio_write8(&mut self, addr: u64, value: u8) {
        // Byte writes not commonly used for display
    }
}

impl Default for D1DisplayEmulated {
    fn default() -> Self {
        Self::new()
    }
}
