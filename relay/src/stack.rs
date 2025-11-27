use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium};
use smoltcp::socket::tcp;
use smoltcp::socket::udp;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A user-space network device that bridges smoltcp with our Relay Switch.
pub struct VirtualDevice {
    pub rx_queue: VecDeque<Vec<u8>>,
    pub tx_queue: VecDeque<Vec<u8>>,
    pub mtu: usize,
}

impl VirtualDevice {
    pub fn new() -> Self {
        Self {
            rx_queue: VecDeque::new(),
            tx_queue: VecDeque::new(),
            mtu: 1500,
        }
    }
}

impl Device for VirtualDevice {
    type RxToken<'a> = Vec<u8>;
    type TxToken<'a> = &'a mut VirtualDevice;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if let Some(frame) = self.rx_queue.pop_front() {
            Some((frame, self))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(self)
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = self.mtu;
        caps.medium = Medium::Ethernet;
        caps
    }
}

impl smoltcp::phy::RxToken for Vec<u8> {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self)
    }
}

impl<'a> smoltcp::phy::TxToken for &'a mut VirtualDevice {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len];
        let result = f(&mut buffer);
        self.tx_queue.push_back(buffer);
        result
    }
}

/// A user-space TCP/IP stack (Slirp) using smoltcp.
/// It performs NAT/Masquerading in user-space.
pub struct UserSpaceStack {
    iface: Interface,
    sockets: SocketSet<'static>,
    device: VirtualDevice,
    
    // Map smoltcp TCP handles to host Tokio TcpStreams
    tcp_nat: HashMap<tcp::SocketHandle, tokio::net::TcpStream>,
}

impl UserSpaceStack {
    pub fn new() -> Self {
        let device = VirtualDevice::new();
        let mut config = Config::new(EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]).into());
        config.random_seed = 0x12345678;
        
        let mut iface = Interface::new(config, &mut VirtualDevice::new(), Instant::now());
        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs.push(IpCidr::new(Ipv4Address::new(10, 0, 2, 2).into(), 24)).unwrap();
        });
        
        iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2).into()).unwrap();

        Self {
            iface,
            sockets: SocketSet::new(vec![]),
            device,
            tcp_nat: HashMap::new(),
        }
    }

    /// Process an incoming frame from a VM.
    pub fn accept_frame(&mut self, frame: Vec<u8>) {
        self.device.rx_queue.push_back(frame);
    }

    /// Poll the stack and return any outgoing frames to be sent to VMs.
    pub fn poll(&mut self) -> Vec<Vec<u8>> {
        let timestamp = Instant::now();
        let _ = self.iface.poll(timestamp, &mut self.device, &mut self.sockets);
        
        let mut frames = Vec::new();
        while let Some(frame) = self.device.tx_queue.pop_front() {
            frames.push(frame);
        }
        frames
    }
}

