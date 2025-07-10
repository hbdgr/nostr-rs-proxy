# Nostr Rust Proxy

A simple and reliable [Nostr](https://github.com/nostr-protocol/nostr) relay proxy server built in Rust using [tokio](https://tokio.rs/) and [tokio-tungstenite](https://docs.rs/tokio-tungstenite).

This proxy provides a unified way to connect to multiple Nostr relays, consolidating their event streams into a single pipeline.

## Features

- **Multiple Relay Connections**  
  Maintains simultaneous connections to multiple Nostr relays, returning the fastest response while ensuring redundancy.

- **Memory Cache**  
  Keeps a short-term in-memory cache of recent events to avoid sending duplicates from multiple relays, helping reduce load on clients.

- **Extensible Architecture**  
  Organized with modular components, making it easier to add features or customize behavior in the future.

## TODOs

- [x] Memory cache: Event deduplication  
- [ ] Cache TTL: Automatic expiration  
- [x] Message handling: Basic REQ/EVENT support  
- [ ] Relay reconnection: Persistent connections  
- [ ] Metrics collection: Event counters  
- [ ] Frontend UI: Monitoring dashboard  

## Configuration

Similar to [nostr-rs-relay](https://sr.ht/~gheartsfield/nostr-rs-relay), the configuration is defined in a `toml` file (e.g., `config.toml`):

```toml
[network]
# Bind to the specified network address
address = "0.0.0.0"

# Listen on the specified port
port = 8085

[sources]
# External sources (Nostr relays)
relays = [
  "ws://0.0.0.0:8080",
  "ws://0.0.0.0:8080",
]
```

- **`network.address`** – The IP address on which the proxy server will listen.  
- **`network.port`** – The port for incoming connections from Nostr clients.  
- **`sources.relays`** – A list of one or more Nostr relay addresses that the proxy will connect to on startup.

## Usage

### Building the Project

```bash
cargo build --release
```

### Running the Proxy Server

#### Production Mode

```bash
RUST_LOG=info cargo run --release
```

#### Debug Mode (more verbose logging)

```bash
RUST_LOG=debug cargo run
```

When the proxy starts, it will connect to the relays specified in your configuration file, listen on the configured address and port, and begin forwarding events to any connected Nostr clients. Make sure to monitor logs for any connection issues.
