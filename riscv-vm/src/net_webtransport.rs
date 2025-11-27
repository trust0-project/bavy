//! WebTransport network backend.
//!
//! This backend tunnels Ethernet frames over WebTransport DATAGRAMs.

use crate::net::NetworkBackend;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread;
    use tokio::runtime::Runtime;
    use wtransport::ClientConfig;
    use wtransport::Endpoint;
    use wtransport::tls::Sha256Digest;

    pub struct WebTransportBackend {
        tx_to_transport: Option<Sender<Vec<u8>>>,
        rx_from_transport: Option<Receiver<Vec<u8>>>,
        mac: [u8; 6],
    }

    impl WebTransportBackend {
        pub fn new(url: &str, cert_hash: Option<String>) -> Self {
            let mut mac = [0x52, 0x54, 0x00, 0x00, 0x00, 0x00];
            // Generate MAC from URL hash
            let hash: u32 = url.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
            mac[3] = ((hash >> 16) & 0xff) as u8;
            mac[4] = ((hash >> 8) & 0xff) as u8;
            mac[5] = (hash & 0xff) as u8;

            // We need to spawn a background thread for the Tokio runtime
            // because wtransport is async.
            let (tx_to_transport, rx_to_transport) = channel::<Vec<u8>>();
            let (tx_from_transport, rx_from_transport) = channel::<Vec<u8>>();

            let url = url.to_string();

            thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async move {
                    let config = if let Some(hash_str) = cert_hash {
                         // Parse hex hash
                        let bytes = hex::decode(hash_str.replace(":", "")).expect("Invalid hex hash");
                        let array: [u8; 32] = bytes.try_into().expect("Invalid hash length");
                        let digest = Sha256Digest::from(array);
                        ClientConfig::builder()
                            .with_bind_default()
                            .with_server_certificate_hashes(vec![digest])
                            .build()
                    } else {
                         ClientConfig::builder()
                            .with_bind_default()
                            .with_no_cert_validation() // INSECURE: For testing only if no hash provided
                            .build()
                    };

                    let endpoint = Endpoint::client(config).unwrap();
                    
                    log::info!("[WebTransport] Connecting to {}...", url);
                    let connection = match endpoint.connect(url).await {
                        Ok(conn) => conn,
                        Err(e) => {
                            log::error!("[WebTransport] Connection failed: {}", e);
                            return;
                        }
                    };
                    log::info!("[WebTransport] Connected!");

                    // Split loop
                    let connection = Arc::new(connection);
                    let conn_send = connection.clone();
                    let conn_recv = connection.clone();

                    // Sender task
                    tokio::spawn(async move {
                        loop {
                            // This is blocking in async context, but we are in a dedicated thread/runtime
                            // so we can't just block the executor. We need to poll the channel.
                            // Ideally we would use tokio::sync::mpsc but the interface is std::sync::mpsc.
                            // For now, we'll use a blocking loop with sleep in a spawn_blocking?
                            // Or better, just use a simple loop here since we are not doing much else.
                            // But we need to read from the std channel.
                            // We can convert std channel to async? No easy way.
                            // We'll poll periodically.
                            if let Ok(data) = rx_to_transport.try_recv() {
                                if let Err(e) = conn_send.send_datagram(data) {
                                    log::error!("Failed to send datagram: {}", e);
                                    break;
                                }
                            } else {
                                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                            }
                        }
                    });

                    // Receiver loop
                    loop {
                         match conn_recv.receive_datagram().await {
                             Ok(datagram) => {
                                 let _ = tx_from_transport.send(datagram.to_vec());
                             }
                             Err(e) => {
                                 log::error!("Receive error: {}", e);
                                 break;
                             }
                         }
                    }
                });
            });

            Self {
                tx_to_transport: Some(tx_to_transport),
                rx_from_transport: Some(rx_from_transport),
                mac,
            }
        }
    }

    impl NetworkBackend for WebTransportBackend {
        fn init(&mut self) -> Result<(), String> {
            // Already initialized in new()
            Ok(())
        }

        fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
            if let Some(rx) = &self.rx_from_transport {
                match rx.try_recv() {
                    Ok(data) => Ok(Some(data)),
                    Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
                    Err(_) => Err("Disconnected".to_string()),
                }
            } else {
                Ok(None)
            }
        }

        fn send(&self, buf: &[u8]) -> Result<(), String> {
            if let Some(tx) = &self.tx_to_transport {
                tx.send(buf.to_vec()).map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err("Not connected".to_string())
            }
        }

        fn mac_address(&self) -> [u8; 6] {
            self.mac
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{WebTransport, WebTransportOptions, WebTransportHash, ReadableStreamDefaultReader, WritableStreamDefaultWriter};
    use js_sys::{Uint8Array, Array};
    use wasm_bindgen_futures::JsFuture;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use std::cell::RefCell;

    pub struct WebTransportBackend {
        url: String,
        cert_hash: Option<String>,
        mac: [u8; 6],
        transport: Option<WebTransport>,
        writer: Option<WritableStreamDefaultWriter>,
        rx_queue: Rc<RefCell<VecDeque<Vec<u8>>>>,
    }

    // WASM is single threaded
    unsafe impl Send for WebTransportBackend {}

    impl WebTransportBackend {
        pub fn new(url: &str, cert_hash: Option<String>) -> Self {
             let mut mac = [0x52, 0x54, 0x00, 0x00, 0x00, 0x00];
            let hash: u32 = url.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
            mac[3] = ((hash >> 16) & 0xff) as u8;
            mac[4] = ((hash >> 8) & 0xff) as u8;
            mac[5] = (hash & 0xff) as u8;

            Self {
                url: url.to_string(),
                cert_hash,
                mac,
                transport: None,
                writer: None,
                rx_queue: Rc::new(RefCell::new(VecDeque::new())),
            }
        }
    }

    impl NetworkBackend for WebTransportBackend {
        fn init(&mut self) -> Result<(), String> {
            let options = WebTransportOptions::new();
            
            if let Some(hash_hex) = &self.cert_hash {
                let bytes = hex::decode(hash_hex.replace(":", "")).map_err(|e| e.to_string())?;
                let array = Uint8Array::from(&bytes[..]);
                
                let hash_obj = WebTransportHash::new();
                hash_obj.set_algorithm("sha-256");
                hash_obj.set_value(&array);
                
                let hashes = Array::new();
                hashes.push(&hash_obj);
                options.set_server_certificate_hashes(&hashes);
            }

            let transport = WebTransport::new_with_options(&self.url, &options)
                .map_err(|e| format!("Failed to create WebTransport: {:?}", e))?;

            let rx_queue = self.rx_queue.clone();

            // Wait for ready
            let transport_clone = transport.clone();
            let ready_promise = transport.ready();
            
            wasm_bindgen_futures::spawn_local(async move {
                match JsFuture::from(ready_promise).await {
                    Ok(_) => {
                        log::info!("WebTransport ready!");
                        
                        // Setup reader
                        let datagrams = transport_clone.datagrams();
                        let readable = datagrams.readable();
                        let reader: ReadableStreamDefaultReader = readable.get_reader().unchecked_into();
                        
                        loop {
                            match JsFuture::from(reader.read()).await {
                                Ok(result) => {
                                    let done = js_sys::Reflect::get(&result, &JsValue::from_str("done")).unwrap().as_bool().unwrap();
                                    if done {
                                        log::info!("WebTransport reader done");
                                        break;
                                    }
                                    let value = js_sys::Reflect::get(&result, &JsValue::from_str("value")).unwrap();
                                    let array = Uint8Array::new(&value);
                                    let data = array.to_vec();
                                    rx_queue.borrow_mut().push_back(data);
                                }
                                Err(e) => {
                                    log::error!("Read error: {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("WebTransport failed to connect: {:?}", e);
                    }
                }
            });

            // Setup writer
            let datagrams = transport.datagrams();
            let writable = datagrams.writable();
            let writer = writable.get_writer().map_err(|e| format!("{:?}", e))?;

            self.transport = Some(transport);
            self.writer = Some(writer);

            Ok(())
        }

        fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
            Ok(self.rx_queue.borrow_mut().pop_front())
        }

        fn send(&self, buf: &[u8]) -> Result<(), String> {
            if let Some(writer) = &self.writer {
                let array = Uint8Array::from(buf);
                let _ = writer.write_with_chunk(&array); // Fire and forget for datagrams
                Ok(())
            } else {
                Err("Not connected".to_string())
            }
        }

        fn mac_address(&self) -> [u8; 6] {
            self.mac
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::WebTransportBackend;

#[cfg(target_arch = "wasm32")]
pub use wasm::WebTransportBackend;

