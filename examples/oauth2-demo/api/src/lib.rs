use ras_jsonrpc_macro::jsonrpc_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request to get current user information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetUserInfoRequest {}

/// Response containing user information
#[derive(Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct GetUserInfoResponse {
    pub user_id: String,
    pub permissions: Vec<String>,
    #[schemars(schema_with = "optional_object_schema")]
    pub metadata: Option<serde_json::Value>,
}

/// Schema function that returns an optional object schema
fn optional_object_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let mut map = serde_json::Map::new();
    map.insert("type".to_string(), serde_json::json!("object"));
    schemars::Schema::from(map)
}

/// Request to create a new document (admin only)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
}

/// Response for document creation
#[derive(Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct CreateDocumentResponse {
    pub document_id: String,
    pub created_at: String,
}

/// Request to list documents
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListDocumentsRequest {
    #[schemars(flatten)]
    pub limit: Option<u32>,
    #[schemars(flatten)]
    pub offset: Option<u32>,
}

/// Document information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DocumentInfo {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub tags: Vec<String>,
}

/// Response for listing documents
#[derive(Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentInfo>,
    pub total: u32,
}

/// Request to delete a document (admin only)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteDocumentRequest {
    pub document_id: String,
}

/// Response for document deletion
#[derive(Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct DeleteDocumentResponse {
    pub success: bool,
    pub message: String,
}

/// Request to get system status (system admin only)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSystemStatusRequest {}

/// System status information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SystemStatus {
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
    pub active_sessions: u32,
    pub version: String,
}

/// Response for system status
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetBetaFeaturesRequest {}

/// Beta feature information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BetaFeature {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Response for beta features
#[derive(Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct GetBetaFeaturesResponse {
    pub features: Vec<BetaFeature>,
}

// Define the JSON-RPC service using the macro
jsonrpc_service!({
    service_name: GoogleOAuth2Service,
    openrpc: true,
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
