# ras-observability-otel

OpenTelemetry implementation for Rust Agent Stack observability, providing production-ready metrics collection with Prometheus export.

## Quick Start

```rust
use ras_observability_otel::standard_setup;

// One-line setup!
let otel = standard_setup("my-service")?;

// Your service now has:
// - Prometheus metrics endpoint at /metrics
// - Request counting and duration tracking
// - User activity monitoring
// - Structured logging integration
```

## Features

- **Zero-config setup**: Sensible defaults out of the box
- **Prometheus integration**: Built-in `/metrics` endpoint
- **Standard metrics**: Request counts, duration histograms, active users
- **Axum integration**: Ready-to-use metrics router
- **Type-safe**: Leverages Rust's type system for safety

## Usage with Service Builders

The observability crates are designed to integrate seamlessly with the REST and JSON-RPC macros:

```rust
// The service builders can use the trackers like this:
let otel = OtelSetupBuilder::new("my-service").build()?;

// Create callbacks for the service builders
let usage_tracker = {
    let tracker = otel.usage_tracker();
    move |headers, user, method, path| {
        let context = RequestContext::rest(method, path);
        async move {
            tracker.track_request(&headers, user.as_ref(), &context).await;
        }
    }
};

// Add to your service
MyServiceBuilder::new()
    .with_usage_tracker(usage_tracker)
    .build()
```

## Metrics Exposed

### Counters
- `requests_started_total`: Total requests initiated
- `requests_completed_total`: Total requests completed (with success status)

### Histograms
- `method_duration_seconds`: Method execution time (only includes method and protocol labels to avoid cardinality explosion)

### Labels
All metrics use minimal labels to prevent cardinality explosion:
- `method`: The method being called (e.g., "GET /users", "createUser")
- `protocol`: REST, JSON-RPC, or WebSocket
- `success`: "true" or "false" (only on completion counters)

**Note**: User attributes are intentionally excluded from all metrics to prevent cardinality explosion. User-specific analysis should be done through logs or dedicated user analytics systems.

## Examples

See the `examples/` directory for:
- `simple_usage.rs`: Basic metrics collection
- `with_rest_service.rs`: Integration with REST services

## Running Examples

```bash
# Simple usage example
cargo run --example simple_usage -p ras-observability-otel

# Then visit http://localhost:3000/metrics
```