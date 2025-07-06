# ras-observability-core

Core traits and types for unified observability across REST and JSON-RPC services in Rust Agent Stack.

## Features

- **Protocol-agnostic**: Works with both REST and JSON-RPC services
- **Type-safe**: Compile-time checked metric definitions
- **Extensible**: Trait-based design allows custom implementations
- **Zero-overhead**: Minimal runtime cost when not in use

## Core Concepts

### RequestContext

A unified representation of requests across different protocols:

```rust
use ras_observability_core::{RequestContext, Protocol};

// For REST services
let context = RequestContext::rest("GET", "/api/v1/users");

// For JSON-RPC services  
let context = RequestContext::jsonrpc("getUser".to_string());

// Add metadata
let context = context.with_metadata("request_id", "12345");
```

### Traits

- `UsageTracker`: Track requests before processing
- `MethodDurationTracker`: Track execution duration
- `ServiceMetrics`: Common metrics interface

## Integration

This crate provides the core abstractions. For a production-ready implementation with OpenTelemetry and Prometheus support, see `ras-observability-otel`.