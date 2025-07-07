use axum::{
    body::Body,
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ras_auth_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_file_macro::file_service;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct UploadResponse {
    file_id: String,
    size: u64,
    filename: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileInfo {
    id: String,
    name: String,
    size: u64,
    content_type: String,
}

// Define the file service
file_service!({
    service_name: DocumentService,
    base_path: "/api/files",
    endpoints: [
        // Public download endpoint
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),

        // Authenticated upload endpoint
        UPLOAD WITH_PERMISSIONS(["upload"]) upload() -> UploadResponse,

        // Admin-only file info endpoint
        DOWNLOAD WITH_PERMISSIONS(["admin"]) info/{file_id: String}() -> FileInfo,
    ]
});

// Simple auth provider for demo
#[derive(Clone)]
struct DemoAuthProvider;

impl AuthProvider for DemoAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            match token.as_str() {
                "user-token" => Ok(AuthenticatedUser {
                    user_id: "user-123".to_string(),
                    permissions: vec!["upload".to_string()]
                        .into_iter()
                        .collect::<HashSet<_>>(),
                    metadata: None,
                }),
                "admin-token" => Ok(AuthenticatedUser {
                    user_id: "admin-456".to_string(),
                    permissions: vec!["upload".to_string(), "admin".to_string()]
                        .into_iter()
                        .collect::<HashSet<_>>(),
                    metadata: None,
                }),
                _ => Err(AuthError::InvalidToken),
            }
        })
    }
}

// Service implementation
#[derive(Clone)]
struct DocumentServiceImpl;

#[async_trait::async_trait]
impl DocumentServiceTrait for DocumentServiceImpl {
    async fn download(
        &self,
        file_id: String,
    ) -> Result<impl IntoResponse, DocumentServiceFileError> {
        // In a real implementation, this would stream from storage
        let content = format!("File content for {}", file_id);
        let body = Body::from(content);

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain")
            .header(
                "content-disposition",
                format!("attachment; filename=\"{}.txt\"", file_id),
            )
            .body(body)
            .unwrap())
    }

    async fn upload(
        &self,
        user: &AuthenticatedUser,
        mut multipart: Multipart,
    ) -> Result<UploadResponse, DocumentServiceFileError> {
        println!("User {} is uploading a file", user.user_id);

        // Process the multipart upload
        while let Some(field) = multipart.next_field().await.map_err(|e| {
            DocumentServiceFileError::UploadFailed(format!("Failed to get next field: {}", e))
        })? {
            let name = field.name().unwrap_or("unknown").to_string();
            let file_name = field.file_name().unwrap_or("unknown").to_string();
            let data = field.bytes().await.map_err(|e| {
                DocumentServiceFileError::UploadFailed(format!("Failed to read field data: {}", e))
            })?;

            println!(
                "Received field '{}' with filename '{}', size: {} bytes",
                name,
                file_name,
                data.len()
            );

            // In a real implementation, you would save this to storage
            return Ok(UploadResponse {
                file_id: format!("file_{}", Uuid::new_v4()),
                size: data.len() as u64,
                filename: file_name,
            });
        }

        Err(DocumentServiceFileError::UploadFailed(
            "No file in multipart data".to_string(),
        ))
    }

    async fn info(
        &self,
        user: &AuthenticatedUser,
        file_id: String,
    ) -> Result<impl IntoResponse, DocumentServiceFileError> {
        println!(
            "Admin {} requesting info for file {}",
            user.user_id, file_id
        );

        // In a real implementation, this would fetch from database
        let info = FileInfo {
            id: file_id.clone(),
            name: format!("{}.pdf", file_id),
            size: 1024 * 1024, // 1MB
            content_type: "application/pdf".to_string(),
        };

        Ok(axum::Json(info))
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create service
    let service = DocumentServiceImpl;
    let auth = DemoAuthProvider;

    // Build the router
    let app = DocumentServiceBuilder::new(service)
        .auth_provider(auth)
        .with_usage_tracker(|headers, method, path| {
            println!("Request: {} {} - Headers: {:?}", method, path, headers);
        })
        .with_duration_tracker(|method, path, duration| {
            println!("Request {} {} took {:?}", method, path, duration);
        })
        .build()
        .layer(TraceLayer::new_for_http());

    println!("File service starting on http://0.0.0.0:3000");
    println!("Try:");
    println!("  curl http://localhost:3000/api/files/download/test123");
    println!(
        "  curl -X POST -H 'Authorization: Bearer user-token' -F 'file=@somefile.txt' http://localhost:3000/api/files/upload"
    );
    println!(
        "  curl -H 'Authorization: Bearer admin-token' http://localhost:3000/api/files/info/test123"
    );

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
