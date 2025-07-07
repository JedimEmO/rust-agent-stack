use ras_file_macro::file_service;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub file_id: String,
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub content_type: Option<String>,
    pub uploaded_at: String,
}

file_service!({
    service_name: DocumentService,
    base_path: "/api/documents",
    body_limit: 104857600, // 100 MB
    endpoints: [
        UPLOAD UNAUTHORIZED upload() -> UploadResponse,
        UPLOAD WITH_PERMISSIONS(["user"]) upload_profile_picture() -> UploadResponse,
        DOWNLOAD UNAUTHORIZED download/{file_id:String}() -> (),
        DOWNLOAD WITH_PERMISSIONS(["user"]) download_secure/{file_id:String}() -> (),
    ]
});

// Re-export the macro-generated WASM client when the feature is enabled
#[cfg(all(target_arch = "wasm32", feature = "wasm-client"))]
pub use wasm_client::*;
