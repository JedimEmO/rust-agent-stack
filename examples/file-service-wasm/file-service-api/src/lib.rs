use ras_file_macro::file_service;
#[cfg(not(target_arch = "wasm32"))]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(JsonSchema))]
pub struct UploadResponse {
    pub file_id: String,
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(JsonSchema))]
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
    openapi: true,
    body_limit: 204857600, // 100 MB
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let doc = generate_documentservice_openapi();
        println!(
            "Generated OpenAPI doc: {}",
            serde_json::to_string_pretty(&doc).unwrap()
        );

        // Test that we can write to file
        generate_documentservice_openapi_to_file().expect("Failed to write OpenAPI to file");
    }
}
