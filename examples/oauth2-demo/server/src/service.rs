use axum::Router;
use oauth2_demo_api::*;
use ras_jsonrpc_core::{AuthProvider, AuthenticatedUser};
use tracing::info;

struct GoogleOAuth2ServiceImpl;

impl GoogleOAuth2ServiceTrait for GoogleOAuth2ServiceImpl {
    async fn get_user_info(
        &self,
        user: &AuthenticatedUser,
        _request: GetUserInfoRequest,
    ) -> Result<GetUserInfoResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!("User {} requested user info", user.user_id);

        Ok(GetUserInfoResponse {
            user_id: user.user_id.clone(),
            permissions: user.permissions.iter().cloned().collect(),
            metadata: user.metadata.clone(),
        })
    }

    async fn list_documents(
        &self,
        user: &AuthenticatedUser,
        request: ListDocumentsRequest,
    ) -> Result<ListDocumentsResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!("User {} requested document list", user.user_id);

        let limit = request.limit.unwrap_or(10).min(100);
        let offset = request.offset.unwrap_or(0);

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
    }

    async fn create_document(
        &self,
        user: &AuthenticatedUser,
        request: CreateDocumentRequest,
    ) -> Result<CreateDocumentResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!("User {} creating document: {}", user.user_id, request.title);

        Ok(CreateDocumentResponse {
            document_id: format!("doc_{}", uuid::Uuid::new_v4()),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn delete_document(
        &self,
        user: &AuthenticatedUser,
        request: DeleteDocumentRequest,
    ) -> Result<DeleteDocumentResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Admin {} deleting document: {}",
            user.user_id, request.document_id
        );

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
    }

    async fn get_system_status(
        &self,
        user: &AuthenticatedUser,
        _request: GetSystemStatusRequest,
    ) -> Result<GetSystemStatusResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!("System admin {} requested system status", user.user_id);

        Ok(GetSystemStatusResponse {
            status: SystemStatus {
                uptime_seconds: 12345,
                memory_usage_mb: 256,
                active_sessions: 42,
                version: "1.0.0-example".to_string(),
            },
        })
    }

    async fn get_beta_features(
        &self,
        user: &AuthenticatedUser,
        _request: GetBetaFeaturesRequest,
    ) -> Result<GetBetaFeaturesResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!("Beta user {} requested beta features", user.user_id);

        Ok(GetBetaFeaturesResponse {
            features: vec![
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
            ],
        })
    }
}

/// Create the API router with all the service handlers
pub fn create_api_router<A: AuthProvider + Clone + Send + Sync + 'static>(
    auth_provider: A,
) -> Router<()> {
    GoogleOAuth2ServiceBuilder::new(GoogleOAuth2ServiceImpl)
        .base_url("/rpc")
        .auth_provider(auth_provider)
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
