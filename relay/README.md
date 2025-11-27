# WebTransport Relay for RISC-V VM

This is a high-performance relay server built with [WebTransport](https://w3c.github.io/webtransport/) and Rust. It enables browser-based RISC-V VMs to communicate with each other and access the external internet via a User-Space NAT gateway.

## Deployment on Docker / Linux

This relay is designed to run in standard Docker containers **without** requiring `NET_ADMIN` capabilities or privileged mode. It uses a user-space NAT implementation for UDP and ICMP.

### Features

- **WebTransport/QUIC:** Uses modern HTTP/3-based transport for low-latency, secure connections over UDP port 4433.
- **Virtual Switch:** Broadcasts Ethernet frames between all connected clients (VMs), effectively placing them on the same virtual LAN.
- **User-Space NAT Gateway:**
    - **Gateway IP:** `10.0.2.2` (responds to ARP and Ping)
    - **External Access:** Allows VMs to ping external hosts (e.g., `8.8.8.8`) and perform UDP queries (e.g., DNS) by proxying traffic through the container's network stack.
    - **No Privileges Needed:** Uses standard UDP sockets and the `ping` command installed in the container.

## Usage

### Running Locally

1. **Install Rust:** Ensure you have the latest stable Rust toolchain installed.
2. **Run the Server:**

```bash
cd relay
cargo run --release
```

The server will start listening on port `4433` (UDP).

On startup, it will print a **Certificate Hash**. You will need this hash to allow the browser to trust the self-signed certificate.

```text
Certificate Hash (use this in client): e7...3f
Listening on https://0.0.0.0:4433
```

### Running with Docker

Build and run the container. Note that no special capabilities are required.

```bash
docker build -t riscv-relay .
docker run -d -p 4433:4433/udp --name relay riscv-relay
```

To verify it's working, check the logs for the certificate hash:

```bash
docker logs relay
```

## Connecting from the Browser

Modern browsers require a secure context (HTTPS or localhost) to use WebTransport. Since we use a self-signed certificate, you must provide the server's certificate hash to the `WebTransport` constructor.

### JavaScript Example

Here is how to connect your web application to the relay:

```javascript
// The hash printed by the relay server on startup
const RELAY_CERT_HASH = "YOUR_CERTIFICATE_HASH_HERE"; 

// Convert hex string to Uint8Array
const certHashBytes = new Uint8Array(
  RELAY_CERT_HASH.match(/.{1,2}/g).map((byte) => parseInt(byte, 16))
);

async function connectToRelay() {
  const url = "https://127.0.0.1:4433";

  const transport = new WebTransport(url, {
    serverCertificateHashes: [
      {
        algorithm: "sha-256",
        value: certHashBytes,
      },
    ],
  });

  try {
    await transport.ready;
    console.log("Connected to relay!");

    // 1. Reader for incoming packets (Ethernet frames)
    const reader = transport.datagrams.readable.getReader();
    
    // 2. Writer for outgoing packets
    const writer = transport.datagrams.writable.getWriter();

    // Example: Receive loop
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      console.log("Received packet:", value); // value is a Uint8Array
    }

  } catch (e) {
    console.error("Connection failed:", e);
  }
}

connectToRelay();
```

### How it Works

1. **Connection:** The browser initiates a WebTransport session over QUIC.
2. **Verification:** The browser verifies the server's self-signed certificate against the provided hash.
3. **Data Exchange:** 
   - The VM encapsulates Ethernet frames into WebTransport datagrams.
   - The Relay receives these datagrams.
4. **Routing:**
   - **Broadcast:** If the packet is internal (e.g., ARP, or destined for another VM), the relay broadcasts it to all other connected clients.
   - **NAT:** If the packet is destined for the internet (e.g., Google DNS `8.8.8.8`), the relay performs NAT, sends it out via the host's UDP socket, and forwards the response back to the specific client.

## Configuration

You can configure the bind address and port via command-line arguments:

```bash
cargo run --release -- --bind 0.0.0.0 --port 4433
```

## Development

To check for compilation errors:

```bash
cargo check
```

To build a release binary:

```bash
cargo build --release
```
