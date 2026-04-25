//! Criterion bench measuring 1 MiB upload + download through the file_service!
//! generated client and router.

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use criterion::{Criterion, criterion_group, criterion_main};
use ras_auth_core::AuthenticatedUser;
use ras_file_macro::file_service;
use ras_test_helpers::{MockAuthProvider, spawn_http};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadResponse {
    file_id: String,
    size: u64,
}

file_service!({
    service_name: BenchSvc,
    base_path: "/files",
    endpoints: [
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),
        UPLOAD WITH_PERMISSIONS(["user"]) upload() -> UploadResponse,
    ]
});

type Storage = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone)]
struct BenchImpl {
    storage: Storage,
}

#[async_trait::async_trait]
impl BenchSvcTrait for BenchImpl {
    async fn download(&self, file_id: String) -> Result<impl IntoResponse, BenchSvcFileError> {
        let bytes = self
            .storage
            .lock()
            .unwrap()
            .iter()
            .find_map(|(id, data)| (id == &file_id).then(|| data.clone()))
            .ok_or(BenchSvcFileError::NotFound)?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(bytes))
            .unwrap())
    }

    async fn upload(
        &self,
        _user: &AuthenticatedUser,
        mut multipart: axum::extract::Multipart,
    ) -> Result<UploadResponse, BenchSvcFileError> {
        let field = multipart
            .next_field()
            .await
            .map_err(|e| BenchSvcFileError::UploadFailed(e.to_string()))?
            .ok_or_else(|| BenchSvcFileError::UploadFailed("no field".into()))?;
        let data = field
            .bytes()
            .await
            .map_err(|e| BenchSvcFileError::UploadFailed(e.to_string()))?;
        let id = format!("file-{}", self.storage.lock().unwrap().len());
        let size = data.len() as u64;
        self.storage
            .lock()
            .unwrap()
            .push((id.clone(), data.to_vec()));
        Ok(UploadResponse { file_id: id, size })
    }
}

fn build_router() -> (axum::Router, Storage) {
    let storage: Storage = Arc::new(Mutex::new(Vec::new()));
    let router = BenchSvcBuilder::<BenchImpl, MockAuthProvider>::new(BenchImpl {
        storage: storage.clone(),
    })
    .auth_provider(MockAuthProvider::default())
    .build();
    (router, storage)
}

fn bench_streaming(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Prepare 1 MiB payload on disk and a live server.
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let payload: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
    tmp.write_all(&payload).unwrap();
    tmp.flush().unwrap();
    let path = tmp.path().to_path_buf();

    let (client, _server) = rt.block_on(async {
        let (router, _storage) = build_router();
        let server = spawn_http(router);
        let base = server.server_address().unwrap();
        let base_str = base.as_str().trim_end_matches('/').to_string();
        let client = BenchSvcClient::builder(base_str)
            .build()
            .expect("client build");
        client.set_bearer_token(Some("user-token".to_string()));
        (client, server)
    });

    c.bench_function("file_upload_download_1mib", |b| {
        b.to_async(&rt).iter(|| {
            let client = &client;
            let path = path.clone();
            async move {
                let r = client
                    .upload(&path, Some("blob.bin"), Some("application/octet-stream"))
                    .await
                    .expect("upload");
                let resp = client.download(r.file_id).await.expect("download");
                let bytes = resp.bytes().await.expect("body");
                std::hint::black_box(bytes);
            }
        });
    });
}

criterion_group!(benches, bench_streaming);
criterion_main!(benches);
