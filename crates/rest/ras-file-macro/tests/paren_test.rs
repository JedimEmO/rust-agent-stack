use axum::body::Body;
use axum::response::{IntoResponse, Response};
use ras_file_macro::file_service;

// Try with parentheses instead of braces
file_service!({
    service_name: TestParen,
    base_path: "/api",
    endpoints: [
        DOWNLOAD UNAUTHORIZED download/{id: String}(),
    ]
});

// Implement the service
#[derive(Clone)]
struct TestService;

#[async_trait::async_trait]
impl TestParenTrait for TestService {
    async fn download(&self, id: String) -> Result<impl IntoResponse, TestParenFileError> {
        Ok(Response::builder()
            .header("content-type", "text/plain")
            .body(Body::from(format!("Download {}", id)))
            .unwrap())
    }
}

#[test]
fn test() {
    let _service = TestService;
}
