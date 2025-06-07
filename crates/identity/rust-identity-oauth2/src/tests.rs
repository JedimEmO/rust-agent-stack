//! Integration and security tests for OAuth2 implementation.

#[cfg(test)]
mod integration_tests {
    use crate::provider::OAuth2Response;
    use crate::{InMemoryStateStore, OAuth2Config, OAuth2Provider, OAuth2ProviderConfig};
    use rust_identity_core::IdentityProvider;
    use std::collections::HashMap;
    use std::sync::Arc;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_mock_oauth_server() -> (MockServer, OAuth2ProviderConfig) {
        let mock_server = MockServer::start().await;

        let provider_config = OAuth2ProviderConfig {
            provider_id: "mock_provider".to_string(),
            client_id: "mock_client_id".to_string(),
            client_secret: "mock_secret".to_string(),
            authorization_endpoint: format!("{}/authorize", mock_server.uri()),
            token_endpoint: format!("{}/token", mock_server.uri()),
            userinfo_endpoint: Some(format!("{}/userinfo", mock_server.uri())),
            redirect_uri: "http://localhost:3000/callback".to_string(),
            scopes: vec!["openid".to_string(), "email".to_string()],
            auth_params: HashMap::new(),
            use_pkce: true,
            user_info_mapping: None,
        };

        (mock_server, provider_config)
    }

    #[tokio::test]
    async fn test_full_oauth2_flow() {
        let (mock_server, provider_config) = setup_mock_oauth_server().await;

        // Mock token endpoint
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "mock_access_token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "refresh_token": "mock_refresh_token",
                "scope": "openid email"
            })))
            .mount(&mock_server)
            .await;

        // Mock userinfo endpoint
        Mock::given(method("GET"))
            .and(path("/userinfo"))
            .and(header("Authorization", "Bearer mock_access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "12345",
                "email": "test@example.com",
                "email_verified": true,
                "name": "Test User",
                "picture": "https://example.com/photo.jpg"
            })))
            .mount(&mock_server)
            .await;

        // Setup provider
        let mut config = OAuth2Config::default();
        config
            .providers
            .insert("mock_provider".to_string(), provider_config);

        let state_store = Arc::new(InMemoryStateStore::new());
        let provider = OAuth2Provider::new(config, state_store);

        // Start OAuth2 flow
        let start_payload = serde_json::json!({
            "type": "StartFlow",
            "provider_id": "mock_provider"
        });

        let start_result = provider.verify(start_payload).await;
        assert!(start_result.is_err());

        let auth_url =
            if let Err(rust_identity_core::IdentityError::ProviderError(json)) = start_result {
                let response: OAuth2Response = serde_json::from_str(&json).unwrap();
                match response {
                    OAuth2Response::AuthorizationUrl { url, state } => {
                        assert!(url.contains("/authorize"));
                        assert!(url.contains("response_type=code"));
                        assert!(url.contains("code_challenge"));
                        state
                    }
                    _ => panic!("Expected authorization URL"),
                }
            } else {
                panic!("Expected provider error with auth URL");
            };

        // Simulate callback
        let callback_payload = serde_json::json!({
            "type": "Callback",
            "provider_id": "mock_provider",
            "code": "mock_auth_code",
            "state": auth_url
        });

        let callback_result = provider.verify(callback_payload).await;
        assert!(callback_result.is_ok());

        let identity = callback_result.unwrap();
        assert_eq!(identity.provider_id, "oauth2:mock_provider");
        assert_eq!(identity.subject, "12345");
        assert_eq!(identity.email, Some("test@example.com".to_string()));
        assert_eq!(identity.display_name, Some("Test User".to_string()));
    }

    #[tokio::test]
    async fn test_oauth2_error_handling() {
        let state_store = Arc::new(InMemoryStateStore::new());
        let config = OAuth2Config::default();
        let provider = OAuth2Provider::new(config, state_store);

        // Test invalid provider
        let payload = serde_json::json!({
            "type": "StartFlow",
            "provider_id": "nonexistent"
        });

        let result = provider.verify(payload).await;
        assert!(result.is_err());

        // Test callback with invalid state
        let payload = serde_json::json!({
            "type": "Callback",
            "provider_id": "google",
            "code": "test_code",
            "state": "invalid_state"
        });

        let result = provider.verify(payload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pkce_security() {
        use crate::client::PkceChallenge;
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        use sha2::{Digest, Sha256};

        let pkce = PkceChallenge::new();

        // Verify that code_challenge is SHA256(code_verifier)
        let mut hasher = Sha256::new();
        hasher.update(pkce.code_verifier.as_bytes());
        let expected_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        assert_eq!(pkce.code_challenge, expected_challenge);
        assert_eq!(pkce.code_challenge_method, "S256");

        // Verify code_verifier meets PKCE requirements (43-128 chars)
        assert!(pkce.code_verifier.len() >= 43);
        assert!(pkce.code_verifier.len() <= 128);
    }

    #[tokio::test]
    async fn test_state_parameter_security() {
        let state_store = Arc::new(InMemoryStateStore::new());
        let mut config = OAuth2Config::default();

        let provider_config = OAuth2ProviderConfig {
            provider_id: "test".to_string(),
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            authorization_endpoint: "https://example.com/auth".to_string(),
            token_endpoint: "https://example.com/token".to_string(),
            userinfo_endpoint: None,
            redirect_uri: "http://localhost:3000/callback".to_string(),
            scopes: vec![],
            auth_params: HashMap::new(),
            use_pkce: false,
            user_info_mapping: None,
        };

        config.providers.insert("test".to_string(), provider_config);
        let provider = OAuth2Provider::new(config, state_store.clone());

        // Generate two authorization URLs
        let payload1 = serde_json::json!({"type": "StartFlow", "provider_id": "test"});
        let payload2 = serde_json::json!({"type": "StartFlow", "provider_id": "test"});

        let result1 = provider.verify(payload1).await;
        let result2 = provider.verify(payload2).await;

        // Extract states
        let state1 = extract_state_from_error(result1);
        let state2 = extract_state_from_error(result2);

        // States should be unique
        assert_ne!(state1, state2);

        // States should be cryptographically random (UUIDs)
        assert_eq!(state1.len(), 36); // UUID v4 format
        assert_eq!(state2.len(), 36);
    }

    fn extract_state_from_error(
        result: Result<rust_identity_core::VerifiedIdentity, rust_identity_core::IdentityError>,
    ) -> String {
        if let Err(rust_identity_core::IdentityError::ProviderError(json)) = result {
            let response: OAuth2Response = serde_json::from_str(&json).unwrap();
            match response {
                OAuth2Response::AuthorizationUrl { state, .. } => state,
                _ => panic!("Expected authorization URL"),
            }
        } else {
            panic!("Expected provider error");
        }
    }

    #[tokio::test]
    async fn test_concurrent_state_handling() {
        use tokio::task;

        let state_store = Arc::new(InMemoryStateStore::new());
        let client = crate::OAuth2Client::new(state_store.clone(), 600, 30);

        let provider_config = OAuth2ProviderConfig {
            provider_id: "test".to_string(),
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            authorization_endpoint: "https://example.com/auth".to_string(),
            token_endpoint: "https://example.com/token".to_string(),
            userinfo_endpoint: None,
            redirect_uri: "http://localhost:3000/callback".to_string(),
            scopes: vec![],
            auth_params: HashMap::new(),
            use_pkce: true,
            user_info_mapping: None,
        };

        // Spawn multiple concurrent authorization requests
        let mut handles = vec![];
        for _ in 0..10 {
            let client = client.clone();
            let config = provider_config.clone();
            let handle = task::spawn(async move {
                client
                    .generate_authorization_url(&config, HashMap::new())
                    .await
            });
            handles.push(handle);
        }

        // Collect all states
        let mut states = vec![];
        for handle in handles {
            let (_, state) = handle.await.unwrap().unwrap();
            states.push(state);
        }

        // All states should be unique
        let unique_states: std::collections::HashSet<_> = states.iter().collect();
        assert_eq!(unique_states.len(), states.len());
    }

    #[tokio::test]
    async fn test_token_exchange_error_cases() {
        let (mock_server, mut provider_config) = setup_mock_oauth_server().await;

        // Test 1: Server returns error
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "The provided authorization code is invalid"
            })))
            .mount(&mock_server)
            .await;

        let state_store = Arc::new(InMemoryStateStore::new());
        let client = crate::OAuth2Client::new(state_store, 600, 30);

        // Store a valid state first
        let state = crate::state::OAuth2State::new(
            "mock_provider".to_string(),
            provider_config.redirect_uri.clone(),
            Some("test_verifier".to_string()),
            600,
        );
        client.state_store().store(state.clone()).await.unwrap();

        let callback = crate::types::AuthorizationResponse {
            code: "invalid_code".to_string(),
            state: state.state,
            error: None,
            error_description: None,
        };

        let result = client.handle_callback(&provider_config, callback).await;
        assert!(result.is_err());

        // Test 2: Malformed token response
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .named("malformed_response")
            .mount(&mock_server)
            .await;

        provider_config.token_endpoint = format!("{}/token", mock_server.uri());

        let state2 = crate::state::OAuth2State::new(
            "mock_provider".to_string(),
            provider_config.redirect_uri.clone(),
            None,
            600,
        );
        client.state_store().store(state2.clone()).await.unwrap();

        let callback2 = crate::types::AuthorizationResponse {
            code: "test_code".to_string(),
            state: state2.state,
            error: None,
            error_description: None,
        };

        let result = client.handle_callback(&provider_config, callback2).await;
        assert!(result.is_err());
    }
}
