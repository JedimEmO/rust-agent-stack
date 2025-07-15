#[cfg(all(feature = "server", feature = "client"))]
mod tests {
    use axum::Router;
    use ras_jsonrpc_macro::jsonrpc_service;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
    struct CreateUserRequest {
        username: String,
        email: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
    struct CreateUserResponse {
        user_id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
    struct GetUserRequest {
        user_id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
    struct GetUserResponse {
        user_id: String,
        username: String,
        email: String,
    }

    jsonrpc_service!({
        service_name: UserService,
        openrpc: true,
        explorer: true,
        methods: [
            UNAUTHORIZED create_user(CreateUserRequest) -> CreateUserResponse,
            WITH_PERMISSIONS(["admin"]) get_user(GetUserRequest) -> GetUserResponse,
        ]
    });

    #[tokio::test]
    async fn test_explorer_routes_generated() {
        // Test that the explorer routes function is generated
        let explorer_routes = userservice_explorer_routes("");

        // The router should have routes for /explorer and /explorer/openrpc.json
        let app = Router::new().merge(explorer_routes);

        // Create test server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn the server
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Test that the explorer page is accessible
        let response = reqwest::get(format!("http://{}/explorer", addr))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);

        let content = response.text().await.unwrap();
        assert!(content.contains("UserService JSON-RPC Explorer"));
        assert!(content.contains("Authentication"));

        // Test that the OpenRPC document is accessible
        let response = reqwest::get(format!("http://{}/explorer/openrpc.json", addr))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);

        let openrpc_doc: serde_json::Value = response.json().await.unwrap();
        assert_eq!(openrpc_doc["info"]["title"], "UserService JSON-RPC API");
        assert!(openrpc_doc["methods"].is_array());
    }

    #[test]
    fn test_explorer_with_custom_path() {
        mod custom_path_service {
            use ras_jsonrpc_macro::jsonrpc_service;

            jsonrpc_service!({
                service_name: TestService,
                openrpc: true,
                explorer: { path: "/api/docs" },
                methods: [
                    UNAUTHORIZED test_method(()) -> String,
                ]
            });

            pub fn test_routes() {
                // Test that the explorer routes function is generated
                let _explorer_routes = testservice_explorer_routes("/api/docs");
            }
        }

        custom_path_service::test_routes();
    }

    #[test]
    fn test_explorer_requires_openrpc() {
        mod no_openrpc_service {
            use ras_jsonrpc_macro::jsonrpc_service;

            jsonrpc_service!({
                service_name: NoOpenRpcService,
                explorer: true,  // This should be ignored without openrpc
                methods: [
                    UNAUTHORIZED test_method(()) -> String,
                ]
            });
        }

        // Explorer routes should not be generated
        // This test just verifies that the macro compiles
    }
}
