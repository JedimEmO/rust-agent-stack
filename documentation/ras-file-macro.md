# ras-file-macro Usage Documentation

The `ras-file-macro` crate provides a powerful procedural macro for building type-safe file upload and download services with built-in authentication, cross-platform client support, and TypeScript bindings.

## Table of Contents
- [Overview](#overview)
- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Macro Syntax](#macro-syntax)
- [Server Implementation](#server-implementation)
- [Client Usage](#client-usage)
- [TypeScript Client Generation](#typescript-client-generation)
- [Authentication and Permissions](#authentication-and-permissions)
- [Error Handling](#error-handling)
- [Advanced Features](#advanced-features)
- [Complete Example](#complete-example)

## Overview

The `file_service!` macro generates:
- A trait for implementing file operations
- Axum router with upload/download endpoints
- Native Rust client with streaming support
- WASM client for browser environments
- TypeScript bindings for type-safe frontend usage
- Built-in authentication and permission handling
- Comprehensive error types

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ras-file-macro = { path = "../../crates/rest/ras-file-macro" }
ras-auth-core = { path = "../../crates/core/ras-auth-core" }
axum = { workspace = true }
tokio = { workspace = true, features = ["full"] }
tower = { workspace = true }

# For WASM client generation
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = { workspace = true }
web-sys = { workspace = true, features = ["File", "FormData", "Blob"] }
```

## Basic Usage

### 1. Define Your File Service

```rust
use ras_file_macro::file_service;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub filename: String,
    pub size: usize,
    pub content_type: String,
}

file_service!({
    service_name: FileStorage,
    base_path: "/api/files",
    body_limit: 52428800,  // 50MB limit
    endpoints: [
        UPLOAD WITH_PERMISSIONS(["upload"]) upload() -> FileMetadata,
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),
    ]
});
```

### 2. Implement the Service Trait

```rust
use axum::response::IntoResponse;
use axum::extract::Multipart;
use async_trait::async_trait;

pub struct MyFileStorage {
    // Your storage implementation
}

#[async_trait]
impl FileStorageTrait for MyFileStorage {
    async fn upload(
        &self,
        user: &AuthenticatedUser,
        mut multipart: Multipart,
    ) -> Result<FileMetadata, FileStorageFileError> {
        // Extract file from multipart
        while let Some(field) = multipart.next_field().await.unwrap() {
            if field.name() == Some("file") {
                let filename = field.file_name()
                    .ok_or(FileStorageFileError::InvalidFormat)?
                    .to_string();
                let content_type = field.content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let data = field.bytes().await
                    .map_err(|e| FileStorageFileError::UploadFailed(e.to_string()))?;
                
                // Store file and return metadata
                let id = uuid::Uuid::new_v4().to_string();
                // ... store file logic ...
                
                return Ok(FileMetadata {
                    id,
                    filename,
                    size: data.len(),
                    content_type,
                });
            }
        }
        Err(FileStorageFileError::InvalidFormat)
    }
    
    async fn download(
        &self,
        file_id: String,
    ) -> Result<impl IntoResponse, FileStorageFileError> {
        // Retrieve file
        let file_path = format!("./uploads/{}", file_id);
        let file = tokio::fs::File::open(&file_path).await
            .map_err(|_| FileStorageFileError::NotFound)?;
        
        let stream = tokio_util::io::ReaderStream::new(file);
        let body = axum::body::Body::from_stream(stream);
        
        Ok(axum::response::Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Content-Disposition", format!("attachment; filename=\"{}\"", file_id))
            .body(body)
            .unwrap())
    }
}
```

### 3. Set Up the Server

```rust
use axum::Router;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let storage = Arc::new(MyFileStorage::new());
    let auth_provider = Arc::new(MyAuthProvider::new());
    
    let file_router = FileStorageBuilder::new(storage)
        .auth_provider(auth_provider)
        .build();
    
    let app = Router::new()
        .merge(file_router);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

## Macro Syntax

```rust
file_service!({
    service_name: ServiceName,
    base_path: "/api/path",
    body_limit: 10485760,  // Optional, in bytes
    endpoints: [
        OPERATION AUTH_REQUIREMENT endpoint_name() -> ResponseType,
        OPERATION AUTH_REQUIREMENT endpoint_name/{param: Type}() -> ResponseType,
    ]
})
```

### Parameters

- **`service_name`**: Name of the service (used for trait and struct generation)
- **`base_path`**: Base URL path for all endpoints
- **`body_limit`** (optional): Maximum upload size in bytes
- **`endpoints`**: List of endpoints with their configuration

### Operations

- **`UPLOAD`**: Creates a POST endpoint accepting multipart/form-data
- **`DOWNLOAD`**: Creates a GET endpoint returning file data

### Authentication Requirements

- **`UNAUTHORIZED`**: No authentication required
- **`WITH_PERMISSIONS(["perm1", "perm2"])`**: Requires any of the listed permissions (OR logic)
- **`WITH_PERMISSIONS([["perm1", "perm2"]])`**: Requires all permissions in inner array (AND logic)

### Path Parameters

Dynamic path segments are supported:
```rust
DOWNLOAD UNAUTHORIZED download/{file_id: String}()
UPLOAD WITH_PERMISSIONS(["admin"]) upload_to/{folder: String}() -> FileMetadata
```

## Server Implementation

### Generated Trait

The macro generates a trait with methods for each endpoint:

```rust
#[async_trait]
pub trait FileStorageTrait: Send + Sync {
    // For UPLOAD endpoints
    async fn upload(
        &self,
        user: &AuthenticatedUser,  // Only if auth required
        param: Type,                // Path parameters if any
        multipart: Multipart
    ) -> Result<ResponseType, FileStorageFileError>;
    
    // For DOWNLOAD endpoints
    async fn download(
        &self,
        user: &AuthenticatedUser,  // Only if auth required
        param: Type,                // Path parameters if any
    ) -> Result<impl IntoResponse, FileStorageFileError>;
}
```

### Service Builder

Configure your service with authentication and observability:

```rust
let router = FileStorageBuilder::new(my_service)
    .auth_provider(auth_provider)
    .usage_callback(|headers, method, path| {
        // Track API usage
    })
    .duration_callback(|method, path, duration| {
        // Track request duration
    })
    .build();
```

### Error Handling

A custom error enum is generated:

```rust
pub enum FileStorageFileError {
    NotFound,
    UploadFailed(String),
    DownloadFailed(String),
    InvalidFormat,
    FileTooLarge,
    Internal(String),
}
```

Each variant maps to appropriate HTTP status codes.

## Client Usage

### Native Rust Client

```rust
// Create client
let client = FileStorageClient::builder("http://localhost:3000")
    .timeout(Duration::from_secs(30))  // Optional
    .build()?;

// Set authentication
client.set_bearer_token(Some("your-jwt-token"));

// Upload file
let metadata = client.upload(
    "/path/to/file.pdf",
    None,  // Optional: override filename
    None   // Optional: override content type
).await?;

// Download file
let response = client.download("file-id-123").await?;
let bytes = response.bytes().await?;
```

### Client Builder Options

```rust
let client = FileStorageClient::builder("http://localhost:3000")
    .client(custom_reqwest_client)  // Optional: custom client
    .timeout(Duration::from_secs(60))  // Optional: request timeout
    .build()?;
```

## TypeScript Client Generation

### 1. Configure for WASM

In your API crate's `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
ras-file-macro = { path = "../path/to/ras-file-macro", features = ["wasm-client"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["File", "FormData"] }
```

### 2. Build WASM Module

Create a `package.json`:

```json
{
  "name": "my-file-api",
  "version": "1.0.0",
  "scripts": {
    "build": "wasm-pack build --target web --out-dir pkg --features wasm-client"
  },
  "devDependencies": {
    "wasm-pack": "^0.12.0"
  }
}
```

Build the module:
```bash
npm run build
```

### 3. Generated TypeScript API

The build process generates TypeScript definitions:

```typescript
// pkg/my_file_api.d.ts
export class WasmFileStorageClient {
  constructor(base_url: string);
  set_bearer_token(token?: string | null): void;
  upload(file: File): Promise<any>;
  download(file_id: string): Promise<any>;
}
```

### 4. Use in TypeScript/JavaScript

```typescript
import init, { WasmFileStorageClient } from './pkg/my_file_api';

// Initialize WASM module
await init();

// Create client
const client = new WasmFileStorageClient('http://localhost:3000');
client.set_bearer_token('your-jwt-token');

// Upload file from input
const fileInput = document.getElementById('file-input') as HTMLInputElement;
const file = fileInput.files[0];
const metadata = await client.upload(file);
console.log('Uploaded:', metadata);

// Download file
const data = await client.download('file-id-123');
const blob = new Blob([data], { type: 'application/octet-stream' });
const url = URL.createObjectURL(blob);
window.open(url);
```

### 5. React/SolidJS Example

```tsx
import { createSignal } from 'solid-js';
import { client } from './lib/client';

export function FileUpload() {
  const [uploading, setUploading] = createSignal(false);
  
  const handleUpload = async (file: File) => {
    setUploading(true);
    try {
      const metadata = await client.upload(file);
      console.log('File uploaded:', metadata);
    } catch (error) {
      console.error('Upload failed:', error);
    } finally {
      setUploading(false);
    }
  };
  
  return (
    <input
      type="file"
      onChange={(e) => {
        const file = e.target.files?.[0];
        if (file) handleUpload(file);
      }}
      disabled={uploading()}
    />
  );
}
```

## Authentication and Permissions

### Integration with ras-auth-core

The macro integrates seamlessly with the `ras-auth-core` authentication system:

```rust
use ras_auth_core::{AuthProvider, AuthenticatedUser};
use std::sync::Arc;

// Use any AuthProvider implementation
let auth_provider: Arc<dyn AuthProvider> = Arc::new(JwtAuthProvider::new(
    session_service,
));

let router = FileStorageBuilder::new(storage)
    .auth_provider(auth_provider)
    .build();
```

### Permission Checking

Permissions are automatically validated before calling your trait methods:

```rust
// Single permission (OR logic)
UPLOAD WITH_PERMISSIONS(["upload", "admin"]) upload() -> FileMetadata

// Multiple permissions required (AND logic)
UPLOAD WITH_PERMISSIONS([["upload", "verified"]]) upload_verified() -> FileMetadata

// Complex permission logic
UPLOAD WITH_PERMISSIONS([["admin"], ["upload", "premium"]]) special_upload() -> FileMetadata
// Requires: admin OR (upload AND premium)
```

### Bearer Token Handling

Tokens are extracted from the `Authorization` header:
```
Authorization: Bearer <your-jwt-token>
```

## Error Handling

### Server-Side Errors

The generated error enum provides semantic error types:

```rust
match result {
    Err(FileStorageFileError::NotFound) => {
        // Handle 404
    }
    Err(FileStorageFileError::FileTooLarge) => {
        // Handle 413
    }
    Err(FileStorageFileError::UploadFailed(msg)) => {
        // Handle upload error
    }
    _ => {}
}
```

### Client-Side Errors

```typescript
try {
  await client.upload(file);
} catch (error) {
  if (error.message.includes('413')) {
    console.error('File too large');
  } else if (error.message.includes('401')) {
    console.error('Authentication required');
  }
}
```

## Advanced Features

### Streaming Large Files

The generated client supports streaming for efficient large file handling:

```rust
// Server implementation
async fn download(&self, file_id: String) -> Result<impl IntoResponse, FileStorageFileError> {
    let file = tokio::fs::File::open(path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);
    
    Ok(Response::builder()
        .header("Content-Type", content_type)
        .body(body)
        .unwrap())
}

// Client usage - stream to file
let mut response = client.download("large-file").await?;
let mut file = tokio::fs::File::create("output.bin").await?;
while let Some(chunk) = response.chunk().await? {
    file.write_all(&chunk).await?;
}
```

### Custom Response Headers

```rust
async fn download(&self, file_id: String) -> Result<impl IntoResponse, _> {
    Ok(Response::builder()
        .header("Content-Type", "application/pdf")
        .header("Content-Disposition", "inline; filename=\"document.pdf\"")
        .header("Cache-Control", "public, max-age=3600")
        .body(body)
        .unwrap())
}
```

### Progress Tracking

```typescript
// TypeScript with progress
const formData = new FormData();
formData.append('file', file);

const xhr = new XMLHttpRequest();
xhr.upload.addEventListener('progress', (e) => {
  if (e.lengthComputable) {
    const percentComplete = (e.loaded / e.total) * 100;
    console.log(`Upload progress: ${percentComplete}%`);
  }
});

xhr.open('POST', `${baseUrl}/api/files/upload`);
xhr.setRequestHeader('Authorization', `Bearer ${token}`);
xhr.send(formData);
```

### Multipart Field Handling

```rust
async fn upload(
    &self,
    user: &AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<FileMetadata, FileStorageFileError> {
    let mut file_data = None;
    let mut metadata = HashMap::new();
    
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("");
        
        match name {
            "file" => {
                let filename = field.file_name().unwrap_or("unnamed").to_string();
                let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
                let data = field.bytes().await?;
                file_data = Some((filename, content_type, data));
            }
            "description" => {
                let value = field.text().await?;
                metadata.insert("description".to_string(), value);
            }
            _ => {}
        }
    }
    
    // Process file_data and metadata
    Ok(FileMetadata { /* ... */ })
}
```

## Complete Example

Here's a complete working example of a file service with authentication:

### API Definition

```rust
// file-api/src/lib.rs
use ras_file_macro::file_service;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct UploadResponse {
    pub id: String,
    pub filename: String,
    pub size: usize,
    pub url: String,
}

file_service!({
    service_name: DocumentService,
    base_path: "/api/documents",
    body_limit: 104857600, // 100 MB
    endpoints: [
        UPLOAD UNAUTHORIZED upload() -> UploadResponse,
        UPLOAD WITH_PERMISSIONS(["user"]) upload_secure() -> UploadResponse,
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),
        DOWNLOAD WITH_PERMISSIONS(["user"]) download_secure/{file_id: String}(),
    ]
});

// Enable WASM client
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
```

### Backend Implementation

```rust
// backend/src/main.rs
use axum::Router;
use tower_http::cors::CorsLayer;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize storage and auth
    let storage = Arc::new(FileSystemStorage::new("./uploads"));
    let auth_provider = Arc::new(JwtAuthProvider::new(/* ... */));
    
    // Build file service
    let file_router = DocumentServiceBuilder::new(storage.clone())
        .auth_provider(auth_provider)
        .usage_callback(|headers, method, path| {
            println!("File API accessed: {} {}", method, path);
        })
        .build();
    
    // Create app with CORS
    let app = Router::new()
        .merge(file_router)
        .layer(CorsLayer::permissive());
    
    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("File service running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
```

### Frontend Usage (TypeScript)

```typescript
// frontend/src/lib/fileClient.ts
import init, { WasmDocumentServiceClient } from '@/pkg/file_api';

let client: WasmDocumentServiceClient | null = null;

export async function getFileClient(): Promise<WasmDocumentServiceClient> {
  if (!client) {
    await init();
    client = new WasmDocumentServiceClient(import.meta.env.VITE_API_URL);
    
    // Set token from auth store
    const token = localStorage.getItem('auth_token');
    if (token) {
      client.set_bearer_token(token);
    }
  }
  return client;
}

// frontend/src/components/FileUpload.tsx
export function FileUpload() {
  const [files, setFiles] = useState<UploadResponse[]>([]);
  
  const handleUpload = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    
    try {
      const client = await getFileClient();
      const response = await client.upload_secure(file);
      setFiles([...files, response]);
    } catch (error) {
      console.error('Upload failed:', error);
    }
  };
  
  const handleDownload = async (fileId: string) => {
    try {
      const client = await getFileClient();
      const data = await client.download_secure(fileId);
      
      // Create download link
      const blob = new Blob([data]);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'file';
      a.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Download failed:', error);
    }
  };
  
  return (
    <div>
      <input type="file" onChange={handleUpload} />
      <ul>
        {files.map(file => (
          <li key={file.id}>
            {file.filename} ({file.size} bytes)
            <button onClick={() => handleDownload(file.id)}>
              Download
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

## Best Practices

1. **File Size Limits**: Always set appropriate `body_limit` values to prevent abuse
2. **Content Type Validation**: Validate file types in your implementation
3. **Virus Scanning**: Consider integrating virus scanning for uploaded files
4. **Storage Strategy**: Use cloud storage (S3, etc.) for production deployments
5. **Cleanup**: Implement file retention policies and cleanup routines
6. **Monitoring**: Use the callback functions to track usage and performance
7. **Security**: Always validate permissions and sanitize file names
8. **CORS**: Configure CORS appropriately for your frontend domains

## Troubleshooting

### Common Issues

1. **"File too large" errors**: Check `body_limit` configuration
2. **CORS errors**: Ensure CORS is configured on the server
3. **Authentication failures**: Verify token format and auth provider setup
4. **WASM build errors**: Ensure `wasm-pack` is installed and features are enabled
5. **TypeScript type errors**: Rebuild WASM module after API changes

### Debug Tips

- Enable debug logging in your auth provider
- Use browser dev tools to inspect multipart requests
- Check server logs for detailed error messages
- Verify file permissions on upload directory

This documentation provides everything needed to implement a production-ready file service using the `ras-file-macro`. The macro handles the complex boilerplate while providing flexibility for custom storage implementations and business logic.