//! Example demonstrating the handler validation feature
//! This example shows what happens when you try to build a service without configuring all handlers

use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use serde::{Deserialize, Serialize};

// Example types
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CalculateRequest {
    a: i32,
    b: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CalculateResponse {
    result: i32,
}

// Mock auth provider
struct DemoAuthProvider;

impl AuthProvider for DemoAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            if token == "demo-token" {
                Ok(AuthenticatedUser {
                    user_id: "demo-user".to_string(),
                    permissions: ["user".to_string()].into_iter().collect(),
                    metadata: None,
                })
            } else {
                Err(AuthError::InvalidToken)
            }
        })
    }
}

// Generate a calculator service
mod calculator_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;

    jsonrpc_service!({
        service_name: CalculatorService,
        methods: [
            UNAUTHORIZED add(CalculateRequest) -> CalculateResponse,
            UNAUTHORIZED subtract(CalculateRequest) -> CalculateResponse,
            WITH_PERMISSIONS(["user"]) multiply(CalculateRequest) -> CalculateResponse,
            WITH_PERMISSIONS(["user"]) divide(CalculateRequest) -> CalculateResponse,
        ]
    });
}

fn main() {
    use calculator_service::*;

    println!("=== JSON-RPC Service Handler Validation Demo ===\n");

    println!("This example demonstrates the handler validation feature.");
    println!("The service builder will panic if not all handlers are configured.\n");

    // Uncomment the following code to see the panic in action:
    /*
    println!("1. Attempting to build service with only 'add' and 'subtract' handlers configured...");
    
    let incomplete_builder = CalculatorServiceBuilder::new("/api/calc")
        .auth_provider(DemoAuthProvider)
        .add_handler(|req| async move {
            Ok(CalculateResponse { result: req.a + req.b })
        })
        .subtract_handler(|req| async move {
            Ok(CalculateResponse { result: req.a - req.b })
        });
    // Note: multiply_handler and divide_handler are NOT configured!

    // This will panic with: "Cannot build service: the following handlers are not configured: multiply, divide"
    let _router = incomplete_builder.build().expect("This should fail!");
    */

    println!("Building service with ALL handlers configured...");
    
    let complete_builder = CalculatorServiceBuilder::new("/api/calc")
        .auth_provider(DemoAuthProvider)
        .add_handler(|req| async move {
            Ok(CalculateResponse { result: req.a + req.b })
        })
        .subtract_handler(|req| async move {
            Ok(CalculateResponse { result: req.a - req.b })
        })
        .multiply_handler(|_user, req| async move {
            Ok(CalculateResponse { result: req.a * req.b })
        })
        .divide_handler(|_user, req| async move {
            if req.b == 0 {
                Err("Division by zero".into())
            } else {
                Ok(CalculateResponse { result: req.a / req.b })
            }
        });

    // This should succeed
    let _router = complete_builder.build().expect("Failed to build complete service");
    println!("✓ Build succeeded! All handlers are configured.");

    println!("\nSummary:");
    println!("- The JSON-RPC service builder now validates that all handlers are configured");
    println!("- If any handler is missing, build() will panic with a helpful error message");
    println!("- This ensures that services are fully configured before deployment");
    println!("- The error message lists exactly which handlers are missing");
    
    println!("\nTo see the panic behavior, uncomment the code block in the source file.");
}