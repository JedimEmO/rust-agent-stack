use ras_file_macro::file_service;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct UploadResponse {
    file_id: String,
    size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileMetadata {
    id: String,
    name: String,
    content_type: String,
    size: u64,
}

// Test basic file service definition
file_service!({
    service_name: TestFileService,
    base_path: "/api/files",
    endpoints: [
        // Upload endpoints
        UPLOAD WITH_PERMISSIONS(["upload"]) upload() -> UploadResponse,
        UPLOAD WITH_PERMISSIONS(["admin"]) upload_document/{doc_type: String}() -> UploadResponse,

        // Download endpoints
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),
        DOWNLOAD WITH_PERMISSIONS(["user"]) download_secure/{file_id: String}(),
        DOWNLOAD WITH_PERMISSIONS([["admin", "moderator"]]) get_metadata/{file_id: String}() -> FileMetadata,
    ]
});

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::Multipart;
    use axum::response::{IntoResponse, Response};
    use ras_auth_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
    use std::collections::HashSet;

    // Mock service implementation
    #[derive(Clone)]
    struct MockFileService;

    #[async_trait::async_trait]
    impl TestFileServiceTrait for MockFileService {
        async fn upload(
            &self,
            _user: &AuthenticatedUser,
            _multipart: Multipart,
        ) -> Result<UploadResponse, TestFileServiceFileError> {
            Ok(UploadResponse {
                file_id: "test-file-123".to_string(),
                size: 1024,
            })
        }

        async fn upload_document(
            &self,
            _user: &AuthenticatedUser,
            doc_type: String,
            _multipart: Multipart,
        ) -> Result<UploadResponse, TestFileServiceFileError> {
            Ok(UploadResponse {
                file_id: format!("doc-{}-456", doc_type),
                size: 2048,
            })
        }

        async fn download(
            &self,
            file_id: String,
        ) -> Result<impl IntoResponse, TestFileServiceFileError> {
            if file_id == "not-found" {
                return Err(TestFileServiceFileError::NotFound);
            }

            let body = Body::from(format!("File content for {}", file_id));
            Ok(Response::builder()
                .header("content-type", "application/octet-stream")
                .header(
                    "content-disposition",
                    format!("attachment; filename=\"{}\"", file_id),
                )
                .body(body)
                .unwrap())
        }

        async fn download_secure(
            &self,
            _user: &AuthenticatedUser,
            file_id: String,
        ) -> Result<impl IntoResponse, TestFileServiceFileError> {
            let body = Body::from(format!("Secure file content for {}", file_id));
            Ok(Response::builder()
                .header("content-type", "application/pdf")
                .body(body)
                .unwrap())
        }

        async fn get_metadata(
            &self,
            _user: &AuthenticatedUser,
            file_id: String,
        ) -> Result<impl IntoResponse, TestFileServiceFileError> {
            let metadata = FileMetadata {
                id: file_id.clone(),
                name: format!("{}.pdf", file_id),
                content_type: "application/pdf".to_string(),
                size: 4096,
            };

            Ok(axum::Json(metadata))
        }
    }

    // Mock auth provider
    #[derive(Clone)]
    struct MockAuthProvider;

    impl AuthProvider for MockAuthProvider {
        fn authenticate(&self, token: String) -> AuthFuture<'_> {
            Box::pin(async move {
                match token.as_str() {
                    "valid-token" => Ok(AuthenticatedUser {
                        user_id: "user-123".to_string(),
                        permissions: vec!["user".to_string(), "upload".to_string()]
                            .into_iter()
                            .collect::<HashSet<_>>(),
                        metadata: None,
                    }),
                    "admin-token" => Ok(AuthenticatedUser {
                        user_id: "admin-456".to_string(),
                        permissions: vec![
                            "user".to_string(),
                            "upload".to_string(),
                            "admin".to_string(),
                        ]
                        .into_iter()
                        .collect::<HashSet<_>>(),
                        metadata: None,
                    }),
                    _ => Err(AuthError::InvalidToken),
                }
            })
        }
    }

    #[test]
    fn test_service_trait_exists() {
        // This test verifies that the trait is generated correctly
        fn assert_trait_impl<T: TestFileServiceTrait>() {}
        assert_trait_impl::<MockFileService>();
    }

    #[test]
    fn test_builder_creation() {
        let service = MockFileService;
        let auth = MockAuthProvider;

        let _builder = TestFileServiceBuilder::new(service)
            .auth_provider(auth)
            .with_usage_tracker(|_headers, method, path| {
                println!("Request: {} {}", method, path);
            })
            .with_duration_tracker(|method, path, duration| {
                println!("Request {} {} took {:?}", method, path, duration);
            });
    }

    #[tokio::test]
    async fn test_client_builder() {
        let client = TestFileServiceClient::builder("http://localhost:3000")
            .build()
            .expect("Failed to build client");

        client.set_bearer_token(Some("test-token"));

        // Test that client methods exist
        assert!(true); // Basic compilation test
    }

    #[test]
    fn test_file_error_variants() {
        // Test that all error variants exist
        let _errors = vec![
            TestFileServiceFileError::NotFound,
            TestFileServiceFileError::UploadFailed("test".to_string()),
            TestFileServiceFileError::DownloadFailed("test".to_string()),
            TestFileServiceFileError::InvalidFormat,
            TestFileServiceFileError::FileTooLarge,
            TestFileServiceFileError::Internal("test".to_string()),
        ];
    }

    #[test]
    fn test_error_into_response() {
        use axum::http::StatusCode;
        use axum::response::IntoResponse;

        let test_cases = vec![
            (TestFileServiceFileError::NotFound, StatusCode::NOT_FOUND),
            (
                TestFileServiceFileError::InvalidFormat,
                StatusCode::BAD_REQUEST,
            ),
            (
                TestFileServiceFileError::FileTooLarge,
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
            (
                TestFileServiceFileError::UploadFailed("test".to_string()),
                StatusCode::BAD_REQUEST,
            ),
            (
                TestFileServiceFileError::DownloadFailed("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                TestFileServiceFileError::Internal("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (error, expected_status) in test_cases {
            let response = error.into_response();
            let (parts, _) = response.into_parts();
            assert_eq!(parts.status, expected_status);
        }
    }
}
