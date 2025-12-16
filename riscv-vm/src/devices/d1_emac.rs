//! Emulated D1 EMAC (Ethernet MAC) Controller
//!
//! Emulates the DWMAC-based Ethernet controller for the VM.
//! Connects to the existing network backend (WebTransport, TAP, etc.)

use std::sync::{Arc, RwLock};

/// EMAC base address (matching D1)
pub const D1_EMAC_BASE: u64 = 0x0450_0000;
pub const D1_EMAC_SIZE: u64 = 0x1000;

// Register offsets (matching D1 hardware)
const EMAC_BASIC_CTL0: u64 = 0x00;
const EMAC_BASIC_CTL1: u64 = 0x04;
const EMAC_INT_STA: u64 = 0x08;
const EMAC_INT_EN: u64 = 0x0C;
const EMAC_TX_CTL0: u64 = 0x10;
const EMAC_TX_CTL1: u64 = 0x14;
const EMAC_TX_DMA_DESC: u64 = 0x20;
const EMAC_RX_CTL0: u64 = 0x24;
const EMAC_RX_CTL1: u64 = 0x28;
const EMAC_RX_DMA_DESC: u64 = 0x34;
const EMAC_MII_CMD: u64 = 0x48;
const EMAC_MII_DATA: u64 = 0x4C;
const EMAC_ADDR_HIGH: u64 = 0x50;
const EMAC_ADDR_LOW: u64 = 0x54;
const EMAC_TX_DMA_STA: u64 = 0xB0;
const EMAC_RX_DMA_STA: u64 = 0xC0;

// PHY registers (emulated)
const PHY_BMCR: u32 = 0x00;
const PHY_BMSR: u32 = 0x01;
const PHY_PHYSID1: u32 = 0x02;
const PHY_PHYSID2: u32 = 0x03;
const PHY_ADVERTISE: u32 = 0x04;
const PHY_LPA: u32 = 0x05;

/// Emulated D1 EMAC controller
pub struct D1EmacEmulated {
    // Registers
    basic_ctl0: u32,
    basic_ctl1: u32,
    int_sta: u32,
    int_en: u32,
    tx_ctl0: u32,
    tx_ctl1: u32,
    rx_ctl0: u32,
    rx_ctl1: u32,
    tx_dma_desc: u32,
    rx_dma_desc: u32,
    mii_cmd: u32,
    mii_data: u32,
    addr_high: u32,
    addr_low: u32,
    
    // PHY state
    phy_bmcr: u32,
    phy_bmsr: u32,
    
    // MAC address
    mac_addr: [u8; 6],
    
    // TX/RX buffers (shared with network backend)
    tx_queue: Arc<RwLock<Vec<Vec<u8>>>>,
    rx_queue: Arc<RwLock<Vec<Vec<u8>>>>,
    
    // Assigned IP address (set by external network backend)
    assigned_ip: Option<[u8; 4]>,
}

impl D1EmacEmulated {
    pub fn new() -> Self {
        Self {
            basic_ctl0: 0,
            basic_ctl1: 0,
            int_sta: 0,
            int_en: 0,
            tx_ctl0: 0,
            tx_ctl1: 0,
            rx_ctl0: 0,
            rx_ctl1: 0,
            tx_dma_desc: 0,
            rx_dma_desc: 0,
            mii_cmd: 0,
            mii_data: 0,
            addr_high: 0,
            addr_low: 0,
            phy_bmcr: 0x1000,  // Auto-neg enabled
            phy_bmsr: 0x786D,  // Link up, 100Mbps capable
            mac_addr: [0x02, 0x00, 0x00, 0x00, 0x00, 0x01],
            tx_queue: Arc::new(RwLock::new(Vec::new())),
            rx_queue: Arc::new(RwLock::new(Vec::new())),
            assigned_ip: None,
        }
    }

    pub fn with_mac(mac: [u8; 6]) -> Self {
        let mut emac = Self::new();
        emac.mac_addr = mac;
        // Initialize MMIO registers so kernel can read the MAC
        // ADDR_LOW: bytes [0..3] = mac[0] | (mac[1] << 8) | (mac[2] << 16) | (mac[3] << 24)
        // ADDR_HIGH: bytes [4..5] = mac[4] | (mac[5] << 8)
        emac.addr_low = (mac[0] as u32)
            | ((mac[1] as u32) << 8)
            | ((mac[2] as u32) << 16)
            | ((mac[3] as u32) << 24);
        emac.addr_high = (mac[4] as u32) | ((mac[5] as u32) << 8);
        emac
    }

    /// Get shared TX queue for network backend
    pub fn tx_queue(&self) -> Arc<RwLock<Vec<Vec<u8>>>> {
        self.tx_queue.clone()
    }

    /// Get shared RX queue for network backend
    pub fn rx_queue(&self) -> Arc<RwLock<Vec<Vec<u8>>>> {
        self.rx_queue.clone()
    }

    /// Queue a packet for reception (from network backend)
    pub fn queue_rx_packet(&self, packet: Vec<u8>) {
        let mut queue = self.rx_queue.write().unwrap();
        queue.push(packet);
    }

    /// Get and clear pending TX packets (for network backend)
    pub fn get_tx_packets(&self) -> Vec<Vec<u8>> {
        let mut queue = self.tx_queue.write().unwrap();
        std::mem::take(&mut *queue)
    }

    /// Set assigned IP address (from network backend/relay)
    pub fn set_ip(&mut self, ip: [u8; 4]) {
        self.assigned_ip = Some(ip);
    }

    /// Get assigned IP address
    pub fn get_ip(&self) -> Option<[u8; 4]> {
        self.assigned_ip
    }

    fn handle_mii_cmd(&mut self) {
        let phy_addr = (self.mii_cmd >> 12) & 0x1F;
        let reg_addr = (self.mii_cmd >> 4) & 0x1F;
        let is_write = (self.mii_cmd & 2) == 0;

        if is_write {
            // MII write
            match reg_addr {
                0 => self.phy_bmcr = self.mii_data,
                4 => {} // Advertise
                _ => {}
            }
        } else {
            // MII read
            self.mii_data = match reg_addr {
                0 => self.phy_bmcr,
                1 => self.phy_bmsr,
                2 => 0x001C,  // RTL8201F ID1
                3 => 0xC816,  // RTL8201F ID2
                4 => 0x01E1,  // Advertise
                5 => 0x45E1,  // LPA (100Mbps FD)
                _ => 0xFFFF,
            };
        }

        // Clear busy bit
        self.mii_cmd &= !(1 << 0);
    }

    /// Check if TX DMA is enabled
    pub fn is_tx_enabled(&self) -> bool {
        (self.tx_ctl0 & (1 << 31)) != 0 && (self.tx_ctl1 & (1 << 30)) != 0
    }

    /// Check if RX DMA is enabled
    pub fn is_rx_enabled(&self) -> bool {
        (self.rx_ctl0 & (1 << 31)) != 0 && (self.rx_ctl1 & (1 << 30)) != 0
    }

    /// Poll DMA descriptors for TX/RX processing
    /// 
    /// This reads TX descriptors from guest DRAM and queues packets for transmission.
    /// It also writes received packets from rx_queue to guest DRAM via RX descriptors.
    ///
    /// DMA Descriptor format (16 bytes):
    /// - status: u32 (bit 31 = OWN, bit 30 = LAST, bit 29 = FIRST)
    /// - size: u32 (TX: length, RX: buffer size << 16 | frame length)
    /// - buf_addr: u32 (physical address of packet buffer)
    /// - next: u32 (physical address of next descriptor)
    pub fn poll_dma(&mut self, dram: &crate::dram::Dram) {
        const DESC_OWN: u32 = 1 << 31;
        const DESC_FIRST: u32 = 1 << 29;
        const DESC_LAST: u32 = 1 << 30;
        const DRAM_BASE: u64 = 0x8000_0000;
        
        // Process TX: Read descriptors, extract packets
        if self.is_tx_enabled() && self.tx_dma_desc != 0 {
            let desc_addr = self.tx_dma_desc as u64;
            if desc_addr >= DRAM_BASE {
                let offset = desc_addr - DRAM_BASE;
                
                // Read descriptor fields
                let status = dram.load_32(offset).unwrap_or(0) as u32;
                
                
                
                // Check if DMA owns this descriptor (driver submitted it)
                if (status & DESC_OWN) != 0 {
                    let size = dram.load_32(offset + 4).unwrap_or(0) as u32;
                    let buf_addr = dram.load_32(offset + 8).unwrap_or(0) as u32;
                    let next_desc = dram.load_32(offset + 12).unwrap_or(0) as u32;
                    
                    let frame_len = (size & 0xFFFF) as usize;
                    
                   
                    
                    if buf_addr >= DRAM_BASE as u32 && frame_len > 0 && frame_len <= 2048 {
                        let buf_offset = (buf_addr as u64) - DRAM_BASE;
                        
                        // Read packet data from buffer
                        if let Ok(packet) = dram.read_range(buf_offset as usize, frame_len) {
                            // Queue for transmission
                            let mut tx = self.tx_queue.write().unwrap();
                            tx.push(packet.clone());
                            
                        }
                    } 
                    // Clear OWN bit to return descriptor to driver
                    let new_status = status & !DESC_OWN;
                    let _ = dram.store_32(offset, new_status as u64);
                    
                    // Move to next descriptor if valid
                    if next_desc >= DRAM_BASE as u32 {
                        self.tx_dma_desc = next_desc;
                    }
                }
            }
        }
        
        // Process RX: Write packets from rx_queue to descriptors
        if self.is_rx_enabled() && self.rx_dma_desc != 0 {
            let desc_addr = self.rx_dma_desc as u64;
            if desc_addr >= DRAM_BASE {
                let offset = desc_addr - DRAM_BASE;
                
                // Read descriptor fields
                let status = dram.load_32(offset).unwrap_or(0) as u32;
                
               
                // Check if descriptor is available (OWN = DMA owns it, ready for RX)
                if (status & DESC_OWN) != 0 {
                    // Check if we have a packet to deliver
                    let packet = {
                        let mut rx = self.rx_queue.write().unwrap();
                        if rx.is_empty() { None } else { Some(rx.remove(0)) }
                    };
                    
                    if let Some(packet) = packet {
                        let size = dram.load_32(offset + 4).unwrap_or(0) as u32;
                        let buf_addr = dram.load_32(offset + 8).unwrap_or(0) as u32;
                        let next_desc = dram.load_32(offset + 12).unwrap_or(0) as u32;
                        
                        let buf_size = ((size >> 16) & 0x1FFF) as usize;  // 13 bits for buffer size
                        let frame_len = packet.len().min(buf_size);
                        
       
                        
                        if buf_addr >= DRAM_BASE as u32 && frame_len > 0 {
                            let buf_offset = (buf_addr as u64) - DRAM_BASE;
                            
                            // Write packet data to buffer
                            for (i, byte) in packet.iter().take(frame_len).enumerate() {
                                let _ = dram.store_8(buf_offset + i as u64, *byte as u64);
                            }
                            
                            // Update descriptor: clear OWN, set frame length, set FIRST/LAST
                            let new_status = DESC_FIRST | DESC_LAST | ((frame_len as u32) << 16);
                            let _ = dram.store_32(offset, new_status as u64);
                            
              
                            
                            // Set interrupt status (RX complete)
                            self.int_sta |= 1 << 8;
                            
                            // Move to next descriptor
                            if next_desc >= DRAM_BASE as u32 {
                                self.rx_dma_desc = next_desc;
                            }
                        }
                    }
                }
            }
        }
    }
}

// MMIO Access Methods (for bus integration)
impl D1EmacEmulated {
    pub fn mmio_read32(&self, addr: u64) -> u32 {
        let offset = addr & 0xFFF;
        
        match offset {
            0x00 => self.basic_ctl0,
            0x04 => self.basic_ctl1,
            0x08 => self.int_sta,
            0x0C => self.int_en,
            0x10 => self.tx_ctl0,
            0x14 => self.tx_ctl1,
            0x20 => self.tx_dma_desc,
            0x24 => self.rx_ctl0,
            0x28 => self.rx_ctl1,
            0x34 => self.rx_dma_desc,
            0x48 => self.mii_cmd,
            0x4C => self.mii_data,
            0x50 => self.addr_high,
            0x54 => self.addr_low,
            0xB0 => 0,  // TX DMA status (idle)
            0xC0 => {
                // RX DMA status - check if packets available
                let queue = self.rx_queue.read().unwrap();
                if queue.is_empty() { 0 } else { 1 }
            }
            // Custom IP config register (for VM relay IP assignment)
            // 0x100: IP address as 32-bit value (big-endian: [3][2][1][0])
            0x100 => {
                let result = if let Some(ip) = self.assigned_ip {
                    ((ip[0] as u32) << 24) | ((ip[1] as u32) << 16) | 
                    ((ip[2] as u32) << 8) | (ip[3] as u32)
                } else {
                    0  // No IP assigned yet
                };
                #[cfg(target_arch = "wasm32")]
                {
                    // Log when kernel reads IP config (first 3 times only to avoid spam)
                    static READ_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                    let count = READ_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if count < 3 {
                        let msg = format!(
                            "[D1 EMAC] mmio_read32(0x100) = 0x{:08x} (IP: {:?})",
                            result, self.assigned_ip
                        );
                        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&msg));
                    }
                }
                result
            }
            _ => 0,
        }
    }

    pub fn mmio_write32(&mut self, addr: u64, value: u32) {
        let offset = addr & 0xFFF;
        
        match offset {
            0x00 => self.basic_ctl0 = value,
            0x04 => {
                if (value & 1) != 0 {
                    // Soft reset
                    self.basic_ctl1 = value & !1;
                } else {
                    self.basic_ctl1 = value;
                }
            }
            0x08 => self.int_sta &= !value,  // Write 1 to clear
            0x0C => self.int_en = value,
            0x10 => self.tx_ctl0 = value,
            0x14 => self.tx_ctl1 = value,
            0x20 => self.tx_dma_desc = value,
            0x24 => self.rx_ctl0 = value,
            0x28 => self.rx_ctl1 = value,
            0x34 => self.rx_dma_desc = value,
            0x48 => {
                self.mii_cmd = value;
                if (value & 1) != 0 {
                    self.handle_mii_cmd();
                }
            }
            0x4C => self.mii_data = value,
            0x50 => self.addr_high = value,
            0x54 => self.addr_low = value,
            _ => {}
        }
    }

    pub fn mmio_read8(&self, addr: u64) -> u8 {
        let word = self.mmio_read32(addr & !3);
        let shift = (addr & 3) * 8;
        (word >> shift) as u8
    }

    #[allow(unused_variables)]
    pub fn mmio_write8(&mut self, addr: u64, value: u8) {
        // Byte writes not commonly used
    }
}

impl Default for D1EmacEmulated {
    fn default() -> Self {
        Self::new()
    }
}
