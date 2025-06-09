# ras-jsonrpc-bidirectional-client

Cross-platform WebSocket client for bidirectional JSON-RPC communication that works on both native and WASM targets.

## Features

- **Cross-platform**: Works on both native (x86_64) and WASM targets
- **JWT Authentication**: Support for JWT tokens via headers or connection parameters
- **Bidirectional Communication**: Send JSON-RPC requests and receive responses, plus handle server notifications
- **Subscription Management**: Subscribe to topics and receive targeted notifications
- **Connection Lifecycle**: Automatic reconnection with configurable backoff strategies
- **Builder Pattern**: Ergonomic client configuration
- **Type Safety**: Leverages the type system for safe JSON-RPC communication

## Platform Support

### Native (x86_64, ARM, etc.)
- Uses `tokio-tungstenite` for high-performance WebSocket communication
- Full async/await support with Tokio runtime
- Supports all standard WebSocket features

### WASM (Browser)
- Uses `web-sys` WebSocket API for browser compatibility
- Compatible with `wasm-bindgen` and web frameworks
- Handles browser-specific WebSocket limitations gracefully

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
ras-jsonrpc-bidirectional-client = { path = "../path/to/crate" }

# For native targets
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.0", features = ["full"] }

# For WASM targets  
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
```

### Basic Usage

```rust
use ras_jsonrpc_bidirectional_client::{Client, ClientBuilder};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and connect client
    let client = ClientBuilder::new("ws://localhost:8080/ws")
        .with_jwt_token("your_jwt_token".to_string())
        .with_auto_connect(true)
        .build()
        .await?;

    // Make a JSON-RPC call
    let response = client.call("get_user_info", Some(json!({"user_id": 123}))).await?;
    println!("Response: {:?}", response);

    // Send a notification (fire-and-forget)
    client.notify("user_activity", Some(json!({"action": "page_view"}))).await?;

    Ok(())
}
```

### Handling Notifications

```rust
use std::sync::Arc;

// Register a notification handler
client.on_notification("user_message", Arc::new(|method, params| {
    println!("Received notification {}: {:?}", method, params);
}));

// Subscribe to a topic
client.subscribe("chat_room_123", Arc::new(|method, params| {
    println!("Chat message: {:?}", params);
})).await?;
```

### Connection Events

```rust
// Handle connection lifecycle events
client.on_connection_event("main", Arc::new(|event| {
    match event {
        ConnectionEvent::Connected { connection_id } => {
            println!("Connected with ID: {}", connection_id);
        }
        ConnectionEvent::Disconnected { reason } => {
            println!("Disconnected: {:?}", reason);
        }
        ConnectionEvent::Reconnecting { attempt } => {
            println!("Reconnecting, attempt: {}", attempt);
        }
        _ => {}
    }
}));
```

### Advanced Configuration

```rust
use ras_jsonrpc_bidirectional_client::{ClientBuilder, ReconnectConfig};
use std::time::Duration;

let reconnect_config = ReconnectConfig::builder()
    .max_attempts(5)
    .initial_delay(Duration::from_secs(1))
    .max_delay(Duration::from_secs(60))
    .backoff_multiplier(2.0)
    .build();

let client = ClientBuilder::new("wss://api.example.com/ws")
    .with_jwt_token("your_token".to_string())
    .with_jwt_in_header(true)
    .with_header("User-Agent", "MyApp/1.0")
    .with_request_timeout(Duration::from_secs(30))
    .with_connection_timeout(Duration::from_secs(10))
    .with_reconnect_config(reconnect_config)
    .with_heartbeat_interval(Some(Duration::from_secs(30)))
    .build()
    .await?;
```

## Authentication

The client supports multiple authentication methods:

### JWT in Authorization Header
```rust
let client = ClientBuilder::new("ws://localhost:8080/ws")
    .with_jwt_token("your_jwt_token".to_string())
    .with_jwt_in_header(true)  // Default
    .build()
    .await?;
```

### JWT as Connection Parameter
```rust
let client = ClientBuilder::new("ws://localhost:8080/ws")
    .with_jwt_token("your_jwt_token".to_string())
    .with_jwt_in_header(false)
    .build()
    .await?;
```

### Custom Headers
```rust
let client = ClientBuilder::new("ws://localhost:8080/ws")
    .with_header("X-API-Key", "your_api_key")
    .with_header("X-Client-Version", "1.0.0")
    .build()
    .await?;
```

## Error Handling

The client provides comprehensive error handling for various scenarios:

```rust
use ras_jsonrpc_bidirectional_client::ClientError;

match client.call("some_method", None).await {
    Ok(response) => {
        // Handle successful response
    }
    Err(ClientError::Timeout { timeout_seconds }) => {
        println!("Request timed out after {}s", timeout_seconds);
    }
    Err(ClientError::NotConnected) => {
        println!("Client is not connected");
        // Attempt to reconnect
        client.connect().await?;
    }
    Err(ClientError::Authentication(msg)) => {
        println!("Authentication failed: {}", msg);
    }
    Err(e) => {
        println!("Other error: {}", e);
    }
}
```

## Connection Management

### Manual Connection Control
```rust
// Connect manually
client.connect().await?;

// Check connection status
if client.is_connected().await {
    println!("Connected with ID: {:?}", client.connection_id().await);
}

// Disconnect
client.disconnect().await?;
```

### Automatic Reconnection
The client supports automatic reconnection with configurable strategies:

- **Exponential backoff**: Delays increase exponentially between attempts
- **Jitter**: Random variation to prevent thundering herd
- **Maximum attempts**: Limit reconnection attempts
- **Connection events**: Get notified of reconnection attempts

## WASM Considerations

When using in WASM environments:

1. **Feature flags**: Use the `wasm` feature for WASM targets
2. **Error handling**: JavaScript errors are wrapped in `ClientError::JavaScript`
3. **Console logging**: Use `console.log` for debugging in browsers
4. **Async runtime**: Use `wasm-bindgen-futures` for Promise integration

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
ras-jsonrpc-bidirectional-client = { path = "...", features = ["wasm"] }
```

## Testing

The crate includes comprehensive tests for both platforms:

```bash
# Test native implementation
cargo test

# Test WASM implementation (requires wasm-pack)
wasm-pack test --node

# Test specific features
cargo test --features native
cargo test --features wasm
```

## Examples

See the `examples/` directory for complete working examples:

- **Basic client**: Simple request/response
- **Subscription example**: Topic-based notifications
- **Authentication example**: JWT and custom auth
- **WASM example**: Browser-based client
- **Reconnection example**: Handling connection failures

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.