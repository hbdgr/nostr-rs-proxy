## [WIP] Nostr Rust Proxy

A blazing-fast [Nostr](https://github.com/nostr-protocol/nostr) relay proxy server built using Rust's Actix library / websocket actors. It provides a unified and efficient way to connect to multiple Nostr relays, consolidating their streams into a single, high-throughput pipeline.

### Features

- Reliable relay connection: Maintains stable connections to multiple Nostr relays, ensuring uninterrupted data flow.

- Memory cache: Employs an in-memory cache to avoid overwhelming clients with duplicate events from different relays, reducing load on clients and simplifying event sorting.

- Extensible architecture: Designed with modularity in mind, allowing for easy integration of additional features and customization.

### TODOs
- [ ] Memory cache
- [ ] Robust reconnection mechanism: Implement a robust mechanism to seamlessly reconnect to configured relays.
- [ ] Frontend UI: Develop a frontend user interface for monitoring and configuring the proxy server.

### Configuration

Inspired by [nostr-rs-relay](https://sr.ht/~gheartsfield/nostr-rs-relay), example:
```
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
This configuration specifies the proxy server's network address and port, as well as the list of Nostr relays to connect. You can adjust these settings as needed to fit your specific environment.

### Usage

## Building the Project
```
cargo build --release
```

## Running the Proxy Server

Run in production mode
```
RUST_LOG=info cargo run --release
```

Run in debug mode (with more logging)
```
RUST_LOG=debug cargo run
```
