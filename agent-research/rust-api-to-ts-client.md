# Rust API to TypeScript Client Generation with ras-file-macro

## Overview

The `ras-file-macro` provides a powerful procedural macro that generates TypeScript/WASM bindgen compatible clients from Rust API definitions. This enables seamless integration between Rust backend services and TypeScript web applications through WebAssembly.

## How It Works

### 1. Macro Definition

The `file_service!` macro (defined in `crates/rest/ras-file-macro/`) takes a service definition and generates:
- Native Rust client implementation
- WASM-bindgen compatible wrapper client
- Server-side implementation
- TypeScript type definitions

Example macro usage:
```rust
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
```

### 2. Code Generation Process

#### WASM Client Generation (`src/client.rs`)

The macro generates a WASM-bindgen wrapper struct that:

1. **Wraps the native client** with `#[wasm_bindgen]` attributes
2. **Conditionally compiles** with `#[cfg(all(target_arch = "wasm32", feature = "wasm-client"))]`
3. **Provides JavaScript-friendly APIs**:
   - Constructor accepting base URL string
   - Authentication method (`set_bearer_token`)
   - Async methods for each endpoint

#### Method Generation

For each endpoint, the macro generates specialized handlers:

**Upload Operations:**
- Accepts JavaScript `File` objects
- Extracts file content as `Uint8Array`
- Reads content type from the File object
- Converts to Rust `Vec<u8>` for processing

**Download Operations:**
- Returns data as JavaScript `Uint8Array`
- Handles binary data transfer efficiently

### 3. Build Process

The WASM client is built using `wasm-pack`:

```bash
wasm-pack build --target web --out-dir pkg --features wasm-client
```

This generates:
- TypeScript definitions (`.d.ts` files)
- JavaScript glue code
- WASM binary module
- Package.json for npm integration

Generated TypeScript interface example:
```typescript
export class WasmDocumentServiceClient {
  constructor(base_url: string);
  set_bearer_token(token?: string | null): void;
  upload(file: File): Promise<any>;
  upload_profile_picture(file: File): Promise<any>;
  download(file_id: string): Promise<any>;
  download_secure(file_id: string): Promise<any>;
}
```

## Architectural Benefits for Web Applications

### 1. Type Safety Across Stack
- Single source of truth for API definitions in Rust
- Automatic TypeScript type generation ensures client-server contract consistency
- Compile-time validation prevents API mismatches

### 2. Performance Advantages
- Direct WASM execution for data processing
- Efficient binary data handling without base64 encoding
- Reduced serialization overhead compared to traditional REST clients

### 3. Developer Experience
- No manual API client maintenance
- Automatic updates when API changes
- IDE autocomplete and type checking in TypeScript

### 4. Authentication Integration
- Built-in bearer token support
- Seamless integration with existing auth systems
- Per-endpoint permission configuration

### 5. Cross-Platform Compatibility
- Same macro generates both native and WASM clients
- Conditional compilation separates platform-specific code
- Server code excluded from WASM builds

## TypeScript Usage

### Client Initialization

```typescript
import init, { WasmDocumentServiceClient } from '@wasm/file_service_api.js';

// Initialize WASM module once
let wasmInitialized = false;
let clientInstance: WasmDocumentServiceClient | null = null;

export async function getClient(): Promise<WasmDocumentServiceClient> {
  if (!wasmInitialized) {
    await init();
    wasmInitialized = true;
  }
  
  if (!clientInstance) {
    clientInstance = new WasmDocumentServiceClient(window.location.origin);
  }
  
  return clientInstance;
}
```

### File Upload Example

```typescript
const client = await getClient();

// Set authentication token if needed
client.set_bearer_token(authToken);

// Upload file directly from input element
const fileInput = document.querySelector('input[type="file"]');
const file = fileInput.files[0];

try {
  const response = await client.upload(file);
  console.log('Upload successful:', response);
} catch (error) {
  console.error('Upload failed:', error);
}
```

### File Download Example

```typescript
const client = await getClient();

try {
  const data = await client.download(fileId);
  
  // Convert Uint8Array to Blob for download
  const blob = new Blob([data], { type: 'application/octet-stream' });
  const url = URL.createObjectURL(blob);
  
  // Trigger download
  const a = document.createElement('a');
  a.href = url;
  a.download = fileName;
  a.click();
  
  URL.revokeObjectURL(url);
} catch (error) {
  console.error('Download failed:', error);
}
```

## Integration with Modern Web Frameworks

The generated client works seamlessly with modern frameworks like:

- **React/Solid.js**: Direct integration with hooks and state management
- **Vue**: Compatible with composition API
- **Svelte**: Works with reactive stores

Example with Solid.js:
```typescript
export default function FileUpload() {
  const [uploading, setUploading] = createSignal(false);
  
  const handleUpload = async (file: File) => {
    setUploading(true);
    try {
      const client = await getClient();
      const response = await client.upload(file);
      // Handle success
    } finally {
      setUploading(false);
    }
  };
  
  // Component JSX...
}
```

## Key Architectural Patterns

### 1. Separation of Concerns
- API definitions in dedicated crate (`file-service-api`)
- Backend implementation separate from client code
- WASM client feature-gated to avoid bloat

### 2. Error Handling
- Rust errors automatically converted to JavaScript exceptions
- Consistent error structure across platforms
- Type-safe error responses

### 3. Resource Management
- Automatic memory management through WASM bindgen
- Efficient binary data transfer
- No manual cleanup required

### 4. Scalability
- Stateless client design
- Connection pooling handled by browser
- Compatible with CDN distribution

## Conclusion

The `ras-file-macro` represents a powerful approach to building type-safe, performant web applications with Rust backends. By generating TypeScript clients through WASM bindgen, it eliminates the traditional pain points of API integration while providing native-like performance for data-intensive operations. This architecture is particularly beneficial for applications dealing with file uploads, binary data processing, or requiring strong type safety across the full stack.