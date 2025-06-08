use axum::Router;
use ras_jsonrpc_core::AuthProvider;
use ras_jsonrpc_macro::jsonrpc_service;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Request to get current user information
#[derive(Debug, Serialize, Deserialize)]
pub struct GetUserInfoRequest {}

/// Response containing user information
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetUserInfoResponse {
    pub user_id: String,
    pub permissions: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to create a new document (admin only)
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
}

/// Response for document creation
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CreateDocumentResponse {
    pub document_id: String,
    pub created_at: String,
}

/// Request to list documents
#[derive(Debug, Serialize, Deserialize)]
pub struct ListDocumentsRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Document information
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub tags: Vec<String>,
}

/// Response for listing documents
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentInfo>,
    pub total: u32,
}

/// Request to delete a document (admin only)
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteDocumentRequest {
    pub document_id: String,
}

/// Response for document deletion
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DeleteDocumentResponse {
    pub success: bool,
    pub message: String,
}

/// Request to get system status (system admin only)
#[derive(Debug, Serialize, Deserialize)]
pub struct GetSystemStatusRequest {}

/// System status information
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
    pub active_sessions: u32,
    pub version: String,
}

/// Response for system status
#[derive(Debug, Serialize, Deserialize)]
pub struct GetSystemStatusResponse {
    pub status: SystemStatus,
}

impl Default for GetSystemStatusResponse {
    fn default() -> Self {
        Self {
            status: SystemStatus {
                uptime_seconds: 0,
                memory_usage_mb: 0,
                active_sessions: 0,
                version: "1.0.0".to_string(),
            },
        }
    }
}

/// Request to access beta features
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBetaFeaturesRequest {}

/// Beta feature information
#[derive(Debug, Serialize, Deserialize)]
pub struct BetaFeature {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Response for beta features
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetBetaFeaturesResponse {
    pub features: Vec<BetaFeature>,
}

// Define the JSON-RPC service using the macro
jsonrpc_service!({
    service_name: GoogleOAuth2Service,
    methods: [
        // Public endpoints (require authentication but no specific permissions)
        WITH_PERMISSIONS([]) get_user_info(GetUserInfoRequest) -> GetUserInfoResponse,
        WITH_PERMISSIONS(["user:read"]) list_documents(ListDocumentsRequest) -> ListDocumentsResponse,

        // Content creation and editing (requires elevated permissions)
        WITH_PERMISSIONS(["content:create"]) create_document(CreateDocumentRequest) -> CreateDocumentResponse,

        // Admin operations
        WITH_PERMISSIONS(["admin:write"]) delete_document(DeleteDocumentRequest) -> DeleteDocumentResponse,

        // System administration
        WITH_PERMISSIONS(["system:admin"]) get_system_status(GetSystemStatusRequest) -> GetSystemStatusResponse,

        // Beta features
        WITH_PERMISSIONS(["beta:access"]) get_beta_features(GetBetaFeaturesRequest) -> GetBetaFeaturesResponse,
    ]
});

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
