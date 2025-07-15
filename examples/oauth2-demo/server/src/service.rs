use axum::Router;
use oauth2_demo_api::*;
use ras_jsonrpc_core::AuthProvider;
use tracing::info;

/// Create the API router with all the service handlers
pub fn create_api_router<A: AuthProvider + Clone + Send + Sync + 'static>(
    auth_provider: A,
) -> Router<()> {
    GoogleOAuth2ServiceBuilder::new("/rpc")
        .auth_provider(auth_provider)
        .get_user_info_handler(|user, _request| async move {
            info!("User {} requested user info", user.user_id);

            Ok(GetUserInfoResponse {
                user_id: user.user_id,
                permissions: user.permissions.into_iter().collect(),
                metadata: user.metadata,
            })
        })
        .list_documents_handler(|user, request| async move {
            info!("User {} requested document list", user.user_id);

            let limit = request.limit.unwrap_or(10).min(100);
            let offset = request.offset.unwrap_or(0);

            // Simulate document data
            let documents = vec![
                DocumentInfo {
                    id: "doc_1".to_string(),
                    title: "Getting Started with OAuth2".to_string(),
                    created_at: "2024-01-15T10:30:00Z".to_string(),
                    tags: vec!["oauth2".to_string(), "authentication".to_string()],
                },
                DocumentInfo {
                    id: "doc_2".to_string(),
                    title: "Advanced JSON-RPC Patterns".to_string(),
                    created_at: "2024-01-16T14:20:00Z".to_string(),
                    tags: vec!["jsonrpc".to_string(), "patterns".to_string()],
                },
                DocumentInfo {
                    id: "doc_3".to_string(),
                    title: "Rust Identity Management".to_string(),
                    created_at: "2024-01-17T09:15:00Z".to_string(),
                    tags: vec!["rust".to_string(), "identity".to_string()],
                },
            ];

            // Apply pagination
            let total = documents.len() as u32;
            let paginated_docs = documents
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();

            Ok(ListDocumentsResponse {
                documents: paginated_docs,
                total,
            })
        })
        .create_document_handler(|user, request| async move {
            info!("User {} creating document: {}", user.user_id, request.title);

            // Simulate document creation
            let document_id = format!("doc_{}", uuid::Uuid::new_v4());
            let created_at = chrono::Utc::now().to_rfc3339();

            Ok(CreateDocumentResponse {
                document_id,
                created_at,
            })
        })
        .delete_document_handler(|user, request| async move {
            info!(
                "Admin {} deleting document: {}",
                user.user_id, request.document_id
            );

            // Simulate document deletion
            if request.document_id.starts_with("doc_") {
                Ok(DeleteDocumentResponse {
                    success: true,
                    message: format!("Document {} deleted successfully", request.document_id),
                })
            } else {
                Ok(DeleteDocumentResponse {
                    success: false,
                    message: "Document not found".to_string(),
                })
            }
        })
        .get_system_status_handler(|user, _request| async move {
            info!("System admin {} requested system status", user.user_id);

            // Simulate system status
            let status = SystemStatus {
                uptime_seconds: 12345,
                memory_usage_mb: 256,
                active_sessions: 42,
                version: "1.0.0-example".to_string(),
            };

            Ok(GetSystemStatusResponse { status })
        })
        .get_beta_features_handler(|user, _request| async move {
            info!("Beta user {} requested beta features", user.user_id);

            let features = vec![
                BetaFeature {
                    name: "Advanced Analytics".to_string(),
                    description: "Enhanced analytics dashboard with real-time metrics".to_string(),
                    enabled: true,
                },
                BetaFeature {
                    name: "AI-Powered Search".to_string(),
                    description: "Semantic search powered by machine learning".to_string(),
                    enabled: user.permissions.contains("feature:preview"),
                },
                BetaFeature {
                    name: "Collaborative Editing".to_string(),
                    description: "Real-time collaborative document editing".to_string(),
                    enabled: false,
                },
            ];

            Ok(GetBetaFeaturesResponse { features })
        })
        .build()
        .expect("Failed to build GoogleOAuth2Service")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ras_jsonrpc_core::{AuthFuture, AuthenticatedUser};
    use std::collections::HashSet;

    // Mock auth provider for testing
    #[derive(Clone)]
    struct MockAuthProvider {
        permissions: HashSet<String>,
    }

    impl MockAuthProvider {
        fn new(permissions: Vec<&str>) -> Self {
            Self {
                permissions: permissions.into_iter().map(|p| p.to_string()).collect(),
            }
        }
    }

    impl AuthProvider for MockAuthProvider {
        fn authenticate(&self, _token: String) -> AuthFuture<'_> {
            let permissions = self.permissions.clone();
            Box::pin(async move {
                Ok(AuthenticatedUser {
                    user_id: "test_user".to_string(),
                    permissions,
                    metadata: None,
                })
            })
        }
    }

    #[test]
    fn test_service_compilation() {
        // This test ensures that the service compiles and can be instantiated
        let auth_provider = MockAuthProvider::new(vec!["user:read"]);
        let _router = create_api_router(auth_provider);
        // If this compiles, the service definition is valid
    }
}
