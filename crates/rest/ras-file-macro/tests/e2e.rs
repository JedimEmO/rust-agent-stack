//! End-to-end test for the file_service! macro: generated reqwest client →
//! axum router → handler. Exercises upload + download with byte-equality and
//! a missing-token rejection.

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ras_auth_core::AuthenticatedUser;
use ras_file_macro::file_service;
use ras_test_helpers::{MockAuthProvider, spawn_http};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadResponse {
    file_id: String,
    size: u64,
}

file_service!({
    service_name: Demo,
    base_path: "/files",
    endpoints: [
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),
        UPLOAD WITH_PERMISSIONS(["user"]) upload() -> UploadResponse,
    ]
});

type Storage = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone)]
struct DemoImpl {
    storage: Storage,
}

#[async_trait::async_trait]
impl DemoTrait for DemoImpl {
    async fn download(&self, file_id: String) -> Result<impl IntoResponse, DemoFileError> {
        let store = self.storage.lock().unwrap();
        let bytes = store
            .iter()
            .find_map(|(id, data)| (id == &file_id).then(|| data.clone()))
            .ok_or(DemoFileError::NotFound)?;

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/octet-stream")
            .body(Body::from(bytes))
            .unwrap())
    }

    async fn upload(
        &self,
        _user: &AuthenticatedUser,
        mut multipart: axum::extract::Multipart,
    ) -> Result<UploadResponse, DemoFileError> {
        let field = multipart
            .next_field()
            .await
            .map_err(|e| DemoFileError::UploadFailed(e.to_string()))?
            .ok_or_else(|| DemoFileError::UploadFailed("no field".into()))?;
        let data = field
            .bytes()
            .await
            .map_err(|e| DemoFileError::UploadFailed(e.to_string()))?;
        let id = format!("file-{}", self.storage.lock().unwrap().len());
        let size = data.len() as u64;
        self.storage
            .lock()
            .unwrap()
            .push((id.clone(), data.to_vec()));
        Ok(UploadResponse { file_id: id, size })
    }
}

fn router(storage: Storage) -> axum::Router {
    DemoBuilder::<DemoImpl, MockAuthProvider>::new(DemoImpl { storage })
        .auth_provider(MockAuthProvider::default())
        .build()
}

fn write_tempfile(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("tempfile");
    f.write_all(bytes).expect("write tempfile");
    f.flush().expect("flush tempfile");
    f
}

#[tokio::test]
async fn upload_and_download_round_trips_bytes() {
    let storage: Storage = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_http(router(storage.clone()));
    let base = server.server_address().unwrap();
    let base_str = base.as_str().trim_end_matches('/').to_string();

    let payload: Vec<u8> = (0u8..=255).cycle().take(64 * 1024).collect();
    let tmp = write_tempfile(&payload);

    let client = DemoClient::builder(base_str.clone())
        .build()
        .expect("client build");
    client.set_bearer_token(Some("user-token".to_string()));

    let upload = client
        .upload(
            tmp.path(),
            Some("blob.bin"),
            Some("application/octet-stream"),
        )
        .await
        .expect("upload ok");
    assert_eq!(upload.size, payload.len() as u64);

    let resp = client.download(upload.file_id).await.expect("download ok");
    let bytes = resp.bytes().await.expect("read body");
    assert_eq!(bytes.as_ref(), payload.as_slice());
}

#[tokio::test]
async fn upload_rejected_without_token() {
    let storage: Storage = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_http(router(storage));
    let base = server.server_address().unwrap();
    let base_str = base.as_str().trim_end_matches('/').to_string();

    let payload = b"hello world";
    let tmp = write_tempfile(payload);

    let client = DemoClient::builder(base_str).build().expect("client build");
    // No bearer token.

    let result = client
        .upload(tmp.path(), Some("hi.txt"), Some("text/plain"))
        .await;
    // The server short-circuits with 401 before consuming the multipart body,
    // so reqwest may surface that either as the parsed status or as a generic
    // connection error depending on how the upload stream was cut. Either is a
    // valid signal of rejection — the only outcome we want to rule out is
    // success.
    assert!(
        result.is_err(),
        "upload must be rejected without a bearer token, got: {result:?}"
    );
}

#[tokio::test]
async fn download_unknown_file_returns_404() {
    let storage: Storage = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_http(router(storage));
    let base = server.server_address().unwrap();
    let base_str = base.as_str().trim_end_matches('/').to_string();

    let client = DemoClient::builder(base_str).build().expect("client build");
    let err = client
        .download("does-not-exist".to_string())
        .await
        .expect_err("missing file must error");
    assert!(err.to_string().contains("404"), "got: {err}");
}
