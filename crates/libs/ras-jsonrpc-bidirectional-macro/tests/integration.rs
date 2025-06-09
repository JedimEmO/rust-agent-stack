//! Integration tests for the bidirectional macro

use ras_jsonrpc_bidirectional_macro::jsonrpc_bidirectional_service;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRequest {
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResponse {
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub message: String,
}

// Test the macro expansion
jsonrpc_bidirectional_service!({
    service_name: TestService,
    client_to_server: [
        UNAUTHORIZED test_method(TestRequest) -> TestResponse,
        WITH_PERMISSIONS(["admin"]) admin_method(String) -> bool,
    ],
    server_to_client: [
        user_notification(NotificationData),
        status_update(String),
    ]
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_code_compiles() {
        // This test ensures that the generated code compiles without errors
        // We can't easily test the runtime behavior without setting up WebSocket connections,
        // but we can ensure the code generation produces valid Rust code.
    }
}
