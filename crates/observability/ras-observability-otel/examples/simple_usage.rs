//! Simple example showing how to use the observability crates with REST services

use axum::{Router, routing::get};
use ras_observability_core::{RequestContext, ServiceMetrics};
use ras_observability_otel::standard_setup;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Set up OpenTelemetry with Prometheus in one line!
    let otel = standard_setup("my-service")?;

    // Example: Track a request manually
    let context = RequestContext::rest("GET", "/api/v1/users");
    otel.metrics().increment_requests_started(&context);

    // Simulate some work
    sleep(Duration::from_millis(100)).await;

    // Track completion
    otel.metrics().increment_requests_completed(&context, true);
    otel.metrics()
        .record_method_duration(&context, Duration::from_millis(100));

    // Create a simple web server with metrics endpoint
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(otel.metrics_router()); // Add metrics endpoint at /metrics

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    println!("Server running on http://localhost:3000");
    println!("Metrics available at http://localhost:3000/metrics");

    axum::serve(listener, app).await?;

    Ok(())
}
