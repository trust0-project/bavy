// MMIO register *values* expected by the xv6 VirtIO driver.
pub const MAGIC_VALUE: u64 = 0x7472_6976;
pub const VERSION: u64 = 2; // Legacy VirtIO MMIO version

pub const VENDOR_ID: u64 = 0x554d_4551;

// Common MMIO register offsets
pub const MAGIC_VALUE_OFFSET: u64 = 0x000;
pub const VERSION_OFFSET: u64 = 0x004;
pub const DEVICE_ID_OFFSET: u64 = 0x008;
pub const VENDOR_ID_OFFSET: u64 = 0x00c;
pub const DEVICE_FEATURES_OFFSET: u64 = 0x010;
pub const DEVICE_FEATURES_SEL_OFFSET: u64 = 0x014;
pub const DRIVER_FEATURES_OFFSET: u64 = 0x020;
pub const DRIVER_FEATURES_SEL_OFFSET: u64 = 0x024;
pub const GUEST_PAGE_SIZE_OFFSET: u64 = 0x028;
pub const QUEUE_SEL_OFFSET: u64 = 0x030;
pub const QUEUE_NUM_MAX_OFFSET: u64 = 0x034;
pub const QUEUE_NUM_OFFSET: u64 = 0x038;
pub const QUEUE_PFN_OFFSET: u64 = 0x040;
pub const QUEUE_READY_OFFSET: u64 = 0x044;
pub const QUEUE_NOTIFY_OFFSET: u64 = 0x050;
pub const INTERRUPT_STATUS_OFFSET: u64 = 0x060;
pub const INTERRUPT_ACK_OFFSET: u64 = 0x064;
pub const STATUS_OFFSET: u64 = 0x070;
pub const QUEUE_DESC_LOW_OFFSET: u64 = 0x080;
pub const QUEUE_DESC_HIGH_OFFSET: u64 = 0x084;
pub const QUEUE_DRIVER_LOW_OFFSET: u64 = 0x090;
pub const QUEUE_DRIVER_HIGH_OFFSET: u64 = 0x094;
pub const QUEUE_DEVICE_LOW_OFFSET: u64 = 0x0a0;
pub const QUEUE_DEVICE_HIGH_OFFSET: u64 = 0x0a4;
pub const CONFIG_GENERATION_OFFSET: u64 = 0x0fc;
pub const CONFIG_SPACE_OFFSET: u64 = 0x100;

// Device IDs
pub const VIRTIO_BLK_DEVICE_ID: u32 = 2;
pub const VIRTIO_NET_DEVICE_ID: u32 = 1;
pub const VIRTIO_RNG_DEVICE_ID: u32 = 4;
#[allow(dead_code)]
pub const VIRTIO_CONSOLE_DEVICE_ID: u32 = 3;

// VirtIO Block Features
#[allow(dead_code)]
pub const VIRTIO_BLK_F_SIZE_MAX: u64 = 1;
#[allow(dead_code)]
pub const VIRTIO_BLK_F_SEG_MAX: u64 = 2;
#[allow(dead_code)]
pub const VIRTIO_BLK_F_GEOMETRY: u64 = 4;
#[allow(dead_code)]
pub const VIRTIO_BLK_F_RO: u64 = 5;
#[allow(dead_code)]
pub const VIRTIO_BLK_F_BLK_SIZE: u64 = 6;
pub const VIRTIO_BLK_F_FLUSH: u64 = 9;

// VirtIO Net Features
pub const VIRTIO_NET_F_MAC: u64 = 5;           // Device has given MAC address
pub const VIRTIO_NET_F_STATUS: u64 = 16;       // Configuration status field available
#[allow(dead_code)]
pub const VIRTIO_NET_F_MRG_RXBUF: u64 = 15;    // Driver can merge receive buffers
#[allow(dead_code)]
pub const VIRTIO_NET_F_CSUM: u64 = 0;          // Device handles checksum
#[allow(dead_code)]
pub const VIRTIO_NET_F_GUEST_CSUM: u64 = 1;    // Driver handles checksum

// VirtIO Net Status bits
pub const VIRTIO_NET_S_LINK_UP: u16 = 1;

pub const QUEUE_SIZE: u32 = 16;

pub const VRING_DESC_F_NEXT: u64 = 1;
pub const VRING_DESC_F_WRITE: u64 = 2;












