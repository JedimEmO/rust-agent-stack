# Basic JSON-RPC Service with Unified Observability

This example demonstrates a basic JSON-RPC service using the new unified observability crates (`ras-observability-core` and `ras-observability-otel`) for production-ready metrics collection.

## Features

- ✅ **JSON-RPC service** with authentication and permissions
- ✅ **Unified observability** using ras-observability-* crates
- ✅ **Prometheus metrics endpoint** at `/metrics`
- ✅ **Automatic metric collection** for RPC requests
- ✅ **Method duration tracking** with low-cardinality labels
- ✅ **Interactive Explorer** - Built-in JSON-RPC Explorer UI
- ✅ **OpenRPC Document** - Auto-generated API specification

## Metrics Collected

1. **`requests_started_total`** (Counter) - Total number of requests started
   - Labels: `method`, `protocol` (JSON-RPC)

2. **`requests_completed_total`** (Counter) - Total number of requests completed
   - Labels: `method`, `protocol`, `success`

3. **`method_duration_seconds`** (Histogram) - Method execution duration
   - Labels: `method`, `protocol` (no user attributes to prevent cardinality explosion)

### Design Principles

This example follows best practices for production metrics:
- **Low cardinality**: No user-specific labels in metrics
- **Meaningful aggregations**: Duration percentiles (P50, P95, P99) are actually useful
- **Cost-effective**: Won't explode your metrics storage
- **User tracking via logs**: User details are logged but not in metrics

## Running the Example

```bash
cargo run -p basic-jsonrpc-service
```

The service will start on `http://localhost:3000` with the following endpoints:

- **JSON-RPC endpoint**: http://localhost:3000/api/rpc
- **JSON-RPC Explorer**: http://localhost:3000/api/explorer
- **Prometheus metrics**: http://localhost:3000/metrics
- **OpenRPC Document**: http://localhost:3000/api/explorer/openrpc.json

## Configuration

### Environment Variables

- `OTLP_ENDPOINT`: The endpoint for the OTLP exporter (default: `http://localhost:4317`)

## Integration with OTLP

While this example uses OpenTelemetry with a Prometheus exporter, you can integrate with OTLP (OpenTelemetry Protocol) backends by using an OpenTelemetry Collector:

### 1. Create a Collector Configuration

Create `collector-config.yaml`:

```yaml
receivers:
  prometheus:
    config:
      scrape_configs:
        - job_name: 'jsonrpc-service'
          scrape_interval: 10s
          static_configs:
            - targets: ['host.docker.internal:3000']  # or 'localhost:3000' if not using Docker

processors:
  batch:

exporters:
  otlp:
    endpoint: "your-otlp-endpoint:4317"  # Replace with your OTLP backend
    tls:
      insecure: true  # Set to false in production with proper certs
  logging:
    verbosity: detailed

service:
  pipelines:
    metrics:
      receivers: [prometheus]
      processors: [batch]
      exporters: [otlp, logging]
```

### 2. Run the Collector

```bash
docker run -p 4317:4317 \
  -v $(pwd)/collector-config.yaml:/etc/otel-collector-config.yaml \
  otel/opentelemetry-collector:latest \
  --config=/etc/otel-collector-config.yaml
```

### 3. Start the Service

The collector will scrape metrics from `http://localhost:3000/metrics` and forward them to your OTLP backend.

## Using the Service

### Example Credentials

- Username: `user`, Password: `password` (basic user)
- Username: `admin`, Password: `secret` (admin user)

### 1. Sign In (Get a Token)

**Admin User:**
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_in",
    "params": {
      "WithCredentials": {
        "username": "admin",
        "password": "secret"
      }
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "Success": {
      "jwt": "admin_token"
    }
  },
  "id": 1
}
```

### 2. Make Authenticated Requests

```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer admin_token" \
  -d '{
    "jsonrpc": "2.0",
    "method": "delete_everything",
    "params": {},
    "id": 2
  }'
```

### 3. Check Metrics

Visit `http://localhost:3000/metrics` to see collected metrics:

```
# HELP rpc_requests_started_total Total number of RPC requests started
# TYPE rpc_requests_started_total counter
rpc_requests_started_total{method="sign_in",user_agent="curl/7.81.0",authenticated="false"} 2
rpc_requests_started_total{method="delete_everything",user_agent="curl/7.81.0",authenticated="true",user_id="admin123",has_admin="true"} 1

# HELP rpc_requests_completed_total Total number of RPC requests completed (Note: This tracks usage_tracker completion, not actual method execution)
# TYPE rpc_requests_completed_total counter
rpc_requests_completed_total{method="sign_in",user_agent="curl/7.81.0",authenticated="false"} 2
rpc_requests_completed_total{method="delete_everything",user_agent="curl/7.81.0",authenticated="true",user_id="admin123",has_admin="true"} 1

# HELP active_users Number of active users
# TYPE active_users counter
active_users{user_type="admin",action="sign_in"} 1
active_users{user_type="user",action="sign_in"} 1
active_users{user_type="user",action="sign_out"} -1
```

## Architecture

The example demonstrates:

1. **Dual Metric Export**: Both push-based (OTLP) and pull-based (Prometheus) metrics
2. **Graceful Fallback**: Continues with Prometheus-only if OTLP collector is unavailable
3. **Request Interception**: Uses `with_usage_tracker` to capture all RPC requests
4. **Rich Labels**: Captures method, authentication status, user info, and user agent

## Integration with Monitoring Systems

### Prometheus

Configure Prometheus to scrape the `/metrics` endpoint:

```yaml
scrape_configs:
  - job_name: 'jsonrpc-service'
    static_configs:
      - targets: ['localhost:3000']
```

### Grafana

Import metrics from either Prometheus or the OTLP collector to visualize:
- Request rates by method and user
- Active user counts
- Authentication success/failure ratios

### Jaeger/Tempo

While this example focuses on metrics, the OTLP setup can be extended to support distributed tracing by adding:
- `opentelemetry-tracing` dependencies
- Trace context propagation
- Span creation in handlers

## Extending the Example

To add custom metrics:

1. **Add to the Metrics struct**:
```rust
struct Metrics {
    // ... existing metrics
    custom_operations: Counter<u64>,
}
```

2. **Initialize in `Metrics::new()`**:
```rust
custom_operations: meter
    .u64_counter("custom_operations_total")
    .with_description("Custom business operations")
    .build(),
```

3. **Record in handlers**:
```rust
metrics.custom_operations.add(1, &[
    KeyValue::new("operation", "important_action"),
]);
```

## Production Considerations

- **OTLP Authentication**: Configure TLS and authentication for secure metric export
- **Cardinality**: Be careful with label values to avoid metric explosion
- **Sampling**: Consider implementing adaptive sampling for high-traffic services
- **Resource Attributes**: Add more service metadata (version, environment, etc.)
- **Error Handling**: Implement proper error tracking metrics
- **Performance**: The metrics collection adds minimal overhead to request processing