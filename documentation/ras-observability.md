# RAS Observability Guide

This guide provides everything you need to add observability to your RAS stack applications using the built-in OpenTelemetry-based observability system.

## Overview

The RAS observability system provides production-ready metrics and monitoring for your applications with:
- Zero-configuration setup with sensible defaults
- Support for REST, JSON-RPC, and WebSocket protocols
- OpenTelemetry metrics with Prometheus export
- Built-in cardinality protection
- Seamless integration with RAS service macros

## Quick Start

Add observability to your service in one line:

```rust
use ras_observability_otel::standard_setup;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize observability with your service name
    let otel = standard_setup("my-service")?;
    
    // Your service now has:
    // - Prometheus metrics endpoint at /metrics
    // - Request and duration tracking
    // - Standard service metrics
    
    // Add the metrics endpoint to your router
    let app = Router::new()
        .route("/api/hello", get(handler))
        .merge(otel.metrics_router()); // Adds /metrics endpoint
    
    // Start your server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

## Metrics Exposed

The observability system automatically exposes the following metrics:

### Counters
- **`requests_started_total`** - Total number of requests initiated
  - Labels: `method`, `protocol`
- **`requests_completed_total`** - Total number of requests completed
  - Labels: `method`, `protocol`, `success` (true/false)

### Histograms
- **`method_duration_seconds`** - Method execution time in seconds
  - Labels: `method`, `protocol`
  - Buckets: 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0 seconds

### Labels
Labels are kept minimal to avoid cardinality explosion:
- **`method`** - The method being called (e.g., "GET /users", "createUser")
- **`protocol`** - One of: "rest", "jsonrpc", "websocket"
- **`success`** - "true" or "false" (only on completion counter)

## Integration with RAS Services

### JSON-RPC Service Integration

The RAS JSON-RPC macro supports automatic observability integration:

```rust
use ras_observability_otel::OtelSetupBuilder;
use ras_jsonrpc_macro::jsonrpc_service;

// Define your service
jsonrpc_service!({
    service_name: MyService,
    methods: [
        UNAUTHORIZED health(()) -> String,
        WITH_PERMISSIONS(["user"]) create_user(CreateUserRequest) -> User,
        WITH_PERMISSIONS(["admin"]) delete_user(DeleteUserRequest) -> bool,
    ]
});

// Implement the service
struct MyServiceImpl;

impl MyService for MyServiceImpl {
    async fn health(&self, _ctx: CallContext, _params: ()) -> Result<String, JsonRpcError> {
        Ok("healthy".to_string())
    }
    
    async fn create_user(&self, _ctx: CallContext, req: CreateUserRequest) -> Result<User, JsonRpcError> {
        // Your implementation
    }
    
    async fn delete_user(&self, _ctx: CallContext, req: DeleteUserRequest) -> Result<bool, JsonRpcError> {
        // Your implementation
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up observability
    let otel = OtelSetupBuilder::new("my-jsonrpc-service").build()?;
    
    // Build your service with observability hooks
    let rpc_router = MyServiceBuilder::new("/rpc")
        .with_usage_tracker({
            let usage_tracker = otel.usage_tracker();
            move |headers, user, payload| {
                let context = RequestContext::jsonrpc(payload.method.clone());
                let usage_tracker = usage_tracker.clone();
                async move {
                    usage_tracker
                        .track_request(&headers, user.as_ref(), &context)
                        .await;
                }
            }
        })
        .with_method_duration_tracker({
            let duration_tracker = otel.method_duration_tracker();
            move |method, user, duration| {
                let context = RequestContext::jsonrpc(method.to_string());
                let duration_tracker = duration_tracker.clone();
                async move {
                    duration_tracker
                        .track_duration(&context, user.as_ref(), duration)
                        .await;
                }
            }
        })
        .build();
    
    // Combine with metrics endpoint
    let app = Router::new()
        .merge(rpc_router)
        .merge(otel.metrics_router());
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### REST Service Integration

For REST services using the RAS REST macro:

```rust
use ras_observability_otel::OtelSetupBuilder;
use ras_rest_macro::rest_service;

// Define your REST service
rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    endpoints: [
        UNAUTHORIZED GET "/health" health() -> HealthResponse,
        WITH_PERMISSIONS(["user"]) GET "/users/:id" get_user(PathParams<UserId>) -> User,
        WITH_PERMISSIONS(["admin"]) DELETE "/users/:id" delete_user(PathParams<UserId>) -> DeleteResponse,
    ]
});

// Implementation...

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up observability
    let otel = OtelSetupBuilder::new("my-rest-service").build()?;
    
    // Build your service with observability hooks
    let app = UserServiceBuilder::new(service_impl)
        .with_usage_tracker({
            let usage_tracker = otel.usage_tracker();
            move |headers, user, method, path| {
                let context = RequestContext::rest(method, path);
                let usage_tracker = usage_tracker.clone();
                async move {
                    usage_tracker
                        .track_request(&headers, user.as_ref(), &context)
                        .await;
                }
            }
        })
        .with_method_duration_tracker({
            let duration_tracker = otel.method_duration_tracker();
            move |method, path, user, duration| {
                let context = RequestContext::rest(method, path);
                let duration_tracker = duration_tracker.clone();
                async move {
                    duration_tracker
                        .track_duration(&context, user.as_ref(), duration)
                        .await;
                }
            }
        })
        .build();
    
    // Add metrics endpoint
    let app = app.merge(otel.metrics_router());
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### WebSocket Service Integration

For bidirectional WebSocket services:

```rust
use ras_observability_otel::OtelSetupBuilder;
use ras_observability_core::{RequestContext, Protocol};

// In your WebSocket connection handler
async fn handle_connection(
    socket: WebSocket,
    otel: Arc<OtelSetup>,
) {
    // Track WebSocket connection
    let context = RequestContext::websocket();
    otel.usage_tracker()
        .track_request(&headers, user.as_ref(), &context)
        .await;
    
    // Handle messages...
    
    // Track individual WebSocket method calls
    let method_context = RequestContext::websocket()
        .with_metadata("method", "sendMessage");
    
    let start = Instant::now();
    // Process message...
    let duration = start.elapsed();
    
    otel.method_duration_tracker()
        .track_duration(&method_context, user.as_ref(), duration)
        .await;
}
```

## Manual Metrics Tracking

For custom metrics or manual tracking outside of the service macros:

```rust
use ras_observability_otel::standard_setup;
use ras_observability_core::RequestContext;
use std::time::{Duration, Instant};

let otel = standard_setup("my-service")?;
let metrics = otel.metrics();

// Track a custom operation
let context = RequestContext::rest("POST", "/api/v1/process");
metrics.increment_requests_started(&context);

let start = Instant::now();
// Do some work...
let success = match do_work().await {
    Ok(_) => true,
    Err(_) => false,
};

// Track completion
metrics.increment_requests_completed(&context, success);
metrics.record_method_duration(&context, start.elapsed());
```

## Advanced Configuration

### Custom Prometheus Registry

```rust
use prometheus::Registry;
use ras_observability_otel::OtelSetupBuilder;

// Create custom registry
let custom_registry = Registry::new();

// Add custom metrics
let custom_counter = prometheus::Counter::new("custom_metric", "Description")?;
custom_registry.register(Box::new(custom_counter.clone()))?;

// Use with observability
let otel = OtelSetupBuilder::new("my-service")
    .with_prometheus_registry(custom_registry)
    .build()?;
```

### Adding Request Metadata

Use metadata for request-specific information that shouldn't be in metrics:

```rust
use ras_observability_core::RequestContext;

let context = RequestContext::rest("POST", "/api/orders")
    .with_metadata("request_id", request_id)
    .with_metadata("customer_id", customer_id)
    .with_metadata("order_type", "express");

// Metadata is included in structured logs but not metrics
otel.usage_tracker()
    .track_request(&headers, user.as_ref(), &context)
    .await;
```

## Production Deployment

### 1. Prometheus Scraping

Configure Prometheus to scrape your service:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'my-service'
    static_configs:
      - targets: ['my-service:3000']
    metrics_path: '/metrics'
```

### 2. OpenTelemetry Collector

For OTLP export, use an OpenTelemetry Collector:

```yaml
# otel-collector-config.yml
receivers:
  prometheus:
    config:
      scrape_configs:
        - job_name: 'my-service'
          static_configs:
            - targets: ['my-service:3000']
          metrics_path: '/metrics'

exporters:
  otlp:
    endpoint: "your-otlp-backend:4317"

service:
  pipelines:
    metrics:
      receivers: [prometheus]
      exporters: [otlp]
```

### 3. Security

Protect the metrics endpoint in production:

```rust
use axum::middleware;
use tower_http::auth::RequireAuthorizationLayer;

let app = Router::new()
    .merge(api_routes)
    .nest(
        "/metrics",
        otel.metrics_router()
            .layer(RequireAuthorizationLayer::bearer("your-metrics-token"))
    );
```

### 4. Dashboards

Example Grafana queries for your dashboards:

```promql
# Request rate by method
rate(requests_completed_total[5m])

# Success rate
sum(rate(requests_completed_total{success="true"}[5m])) /
sum(rate(requests_completed_total[5m]))

# P95 latency by method
histogram_quantile(0.95, 
  sum(rate(method_duration_seconds_bucket[5m])) by (method, le)
)

# Error rate by protocol
sum(rate(requests_completed_total{success="false"}[5m])) by (protocol)
```

## Best Practices

1. **Use standard context types**: Always use `RequestContext::rest()`, `RequestContext::jsonrpc()`, or `RequestContext::websocket()` for consistency.

2. **Avoid custom labels**: Keep user-specific or high-cardinality data in structured logs, not metrics.

3. **Let macros handle integration**: Use the built-in hooks in RAS service macros when possible.

4. **Monitor cardinality**: Keep an eye on your metric cardinality in production.

5. **Use metadata wisely**: Add request-specific data as metadata for correlation in logs.

## Troubleshooting

### Metrics not appearing

1. Check that the metrics endpoint is accessible:
   ```bash
   curl http://localhost:3000/metrics
   ```

2. Verify the OtelSetup is initialized before handling requests

3. Ensure trackers are properly wired to your service

### High cardinality warnings

If you see warnings about high cardinality:
1. Review your method names - they should be generic (e.g., "GET /users/:id" not "GET /users/123")
2. Avoid adding custom labels
3. Use metadata instead of labels for request-specific data

### Missing authentication info

The system tracks authenticated vs anonymous requests. Ensure your `AuthProvider` is properly configured and returning user information.

## Examples

Complete working examples are available in the repository:
- `examples/basic-jsonrpc-service` - JSON-RPC service with metrics
- `examples/rest-service-example` - REST API with Prometheus metrics
- `crates/observability/ras-observability-otel/examples/` - Standalone examples

## Dependencies

Add these to your `Cargo.toml`:

```toml
[dependencies]
ras-observability-core = { path = "../crates/core/ras-observability-core" }
ras-observability-otel = { path = "../crates/observability/ras-observability-otel" }
```

The observability system is designed to be lightweight with minimal dependencies while providing production-ready metrics for your RAS stack applications.