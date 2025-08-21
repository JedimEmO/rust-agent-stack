# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the Rust Agent Stack.

## Quick Reference

### Development Commands
```bash
# Core commands
cargo build                    # Build entire workspace
cargo test                     # Run all tests
cargo fmt && cargo clippy      # Format and lint

# Run examples
cargo run -p basic-jsonrpc     # JSON-RPC service
cargo run -p oauth2-demo       # OAuth2 example
cargo run -p rest-api-demo     # REST API example
cargo run -p file-service-example  # File upload/download

# Build examples
cd examples/wasm-ui-demo && ./build.sh  # Dominator UI
cd examples/rest-wasm-example/typescript-example && npm run dev  # REST TypeScript client
```

## Architecture Overview

Rust Agent Stack (RAS) is a comprehensive framework for building type-safe, authenticated distributed systems with JSON-RPC, REST APIs, and file services.

### 🏗️ Project Structure
```
crates/
├── core/              # Shared traits and types
│   ├── ras-auth-core
│   ├── ras-identity-core
│   └── ras-observability-core
├── rpc/               # JSON-RPC implementation
│   ├── ras-jsonrpc-types
│   ├── ras-jsonrpc-core
│   ├── ras-jsonrpc-macro
│   └── bidirectional/
├── rest/              # REST API implementation
│   ├── ras-rest-core
│   ├── ras-rest-macro
│   └── ras-file-macro # NEW: File upload/download
├── identity/          # Auth providers
│   ├── ras-identity-local
│   ├── ras-identity-oauth2
│   └── ras-identity-session
├── observability/
│   └── ras-observability-otel
├── specs/
│   └── openrpc-types
└── tools/
    └── openrpc-to-bruno
```

### 🎯 Key Features

1. **Type-Safe Service Macros**
   - `jsonrpc_service!` - JSON-RPC services with OpenRPC docs
   - `rest_service!` - REST APIs with OpenAPI 3.0 docs
   - `file_service!` - File upload/download with streaming
   - `jsonrpc_bidirectional_service!` - WebSocket bidirectional RPC

2. **TypeScript Client Generation**
   - REST APIs: OpenAPI spec → TypeScript client via openapi-ts
   - JSON-RPC: WASM compilation for TypeScript bindings
   - Type-safe API calls with full IntelliSense
   - Automatic bearer token management
   - Works in browsers and Node.js

3. **Flexible Authentication**
   - Pluggable `IdentityProvider` trait
   - Built-in providers: Local (Argon2), OAuth2, JWT
   - Permission-based access control
   - Security best practices (timing attack resistance, etc.)

4. **Production-Ready Features**
   - OpenTelemetry metrics with Prometheus export
   - API documentation generation (OpenRPC/OpenAPI)
   - Structured error handling
   - Request rate limiting

## 💻 Development Guidelines

### Workspace Management
```toml
# Root Cargo.toml - Add shared deps here
[workspace.dependencies]
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }

# Crate Cargo.toml - Reference workspace deps
[dependencies]
axum = { workspace = true }
ras-auth-core = { path = "../core/ras-auth-core" }
```

### Critical Rules
1. **Test Immediately**: Run `cargo build` after every change
2. **Use Workspace Deps**: Never duplicate dependency versions
3. **Minimal Dependencies**: Core crates should have minimal deps
4. **Macro-Only Crates**: Procedural macros can ONLY export macros

### ⚠️ Common Pitfalls

| Issue | Wrong ❌ | Correct ✅ |
|-------|---------|------------|
| Router Nesting | `.merge(router.nest("/api", ...))` | `.nest("/api", router)` |
| Bidirectional Macro | `openrpc: true` in bidirectional | Remove `openrpc` field |
| Generated Names | Expect `ChatService` trait | Actually `ChatServiceService` |
| String Types | Mix `String` and `&str` in builders | Check bon builder types |
| Module Exports | Private module items | Add `pub` to exports |

## 📚 Service Macro Examples

### JSON-RPC Service
```rust
jsonrpc_service!({
    service_name: TaskService,
    openrpc: true,  // Generates OpenRPC docs
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS(["user"]) create_task(CreateTaskRequest) -> Task,
        WITH_PERMISSIONS(["admin"]) delete_all_tasks(()) -> (),
    ]
});
```

### REST API Service
```rust
rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,  // Swagger UI at /api/v1/docs
    endpoints: [
        GET UNAUTHORIZED users() -> UsersResponse,
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> User,
        GET WITH_PERMISSIONS(["user"]) users/{id: String}() -> User,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: String}() -> (),
        
        // Query parameters support (NEW!)
        GET UNAUTHORIZED search/users ? q: String & limit: Option<u32> & offset: Option<u32> () -> UsersResponse,
        GET WITH_PERMISSIONS(["user"]) posts ? tag: Option<String> & published: Option<bool> () -> PostsResponse,
        POST WITH_PERMISSIONS(["admin"]) users ? notify: bool (CreateUserRequest) -> User,
        GET UNAUTHORIZED users/{id: String}/posts ? page: u32 & per_page: Option<u32> () -> PostsResponse,
    ]
});
```

#### Query Parameters Syntax
- Place query params after path: `path/{param} ? query1: Type & query2: Type`
- Separate multiple params with `&` (matching URL query string syntax)
- Required params: `param: Type`
- Optional params: `param: Option<Type>`
- Works with auth, path params, and request bodies

### File Service
```rust
file_service!({
    service_name: DocumentService,
    base_path: "/api/documents",
    body_limit: 52428800,  // 50MB
    endpoints: [
        UPLOAD WITH_PERMISSIONS(["user"]) upload() -> FileMetadata,
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),
    ]
});
```

### Bidirectional WebSocket
```rust
jsonrpc_bidirectional_service!({
    service_name: ChatService,
    // NO openrpc field here!
    client_to_server: [
        WITH_PERMISSIONS(["user"]) send_message(SendMessageRequest) -> SendMessageResponse,
    ],
    server_to_client: [
        message_received(MessageReceivedNotification),
    ]
});
```

## 🧪 Testing & Security

### Security Checklist
- [ ] Timing attack resistance (constant-time auth)
- [ ] Username enumeration prevention
- [ ] Rate limiting (5 concurrent auth attempts)
- [ ] Argon2 password hashing
- [ ] JWT expiration and revocation
- [ ] PKCE for OAuth2 flows

### Testing Strategy
1. **Unit Tests**: Test individual components
2. **Integration Tests**: Test crate interactions
3. **E2E Tests**: Test complete auth flows
4. **Macro Tests**: Test generated code with real routes

## 🔐 Authentication Architecture

### Two-Stage Flow
```mermaid
graph LR
    A[Credentials] --> B[IdentityProvider]
    B --> C[VerifiedIdentity]
    C --> D[SessionService]
    D --> E[JWT Token]
    E --> F[AuthProvider]
    F --> G[Protected Service]
```

### Identity Providers
- **Local**: Username/password with Argon2
- **OAuth2**: Google/GitHub with PKCE
- **Session**: JWT management with revocation

### Permission Models
```rust
// Single permission (OR logic)
WITH_PERMISSIONS(["admin", "moderator"])

// Multiple required (AND logic)
WITH_PERMISSIONS([["verified", "premium"]])

// Complex combinations
WITH_PERMISSIONS([["admin"], ["user", "verified"]])
// Requires: admin OR (user AND verified)
```

## 📦 TypeScript Client Generation

### REST API TypeScript Clients (Recommended)

The preferred approach for REST APIs is to generate TypeScript clients from OpenAPI specifications:

1. **Enable OpenAPI Generation**
```rust
rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,  // Enables OpenAPI generation
    serve_docs: true,
    // ... endpoints
});
```

2. **Generate OpenAPI at Compile Time**
```rust
// In backend's build.rs
fn main() {
    // This generates the OpenAPI spec during compilation
    rest_api::generate_userservice_openapi_to_file()
        .expect("Failed to generate OpenAPI spec");
}
```

3. **Configure TypeScript Generation**
```typescript
// openapi-ts.config.ts
import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  client: '@hey-api/client-fetch',
  input: '../backend/target/openapi/userservice.json',
  output: {
    path: './src/generated',
    format: 'prettier',
  },
});
```

4. **Use Generated Client**
```typescript
import * as api from './generated/services.gen';
import type { User, CreateUserRequest } from './generated/types.gen';

// Make type-safe API calls with named methods
const response = await api.getUsers({
  baseUrl: 'http://localhost:3000/api/v1',
});

if (response.data) {
  const users = response.data.users;
}

// Get specific user
const userResponse = await api.getUsersId({
  baseUrl: 'http://localhost:3000/api/v1',
  path: { id: '123' }
});

// POST with typed body and auth
const newUser: CreateUserRequest = {
  name: 'Alice',
  email: 'alice@example.com'
};

const created = await api.postUsers({
  baseUrl: 'http://localhost:3000/api/v1',
  headers: {
    Authorization: 'Bearer your-token'
  },
  body: newUser
});
```

### JSON-RPC WASM Clients

For JSON-RPC services, WASM client generation is still supported:

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]

[features]
wasm-client = ["wasm-bindgen"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
```

```bash
# Build WASM client
wasm-pack build --target web --features wasm-client
```

```typescript
import init, { WasmTaskServiceClient } from './pkg/my_api';

// Initialize WASM
await init();

// Create client
const client = new WasmTaskServiceClient('http://localhost:3000');
client.set_bearer_token('jwt-token');

// Make RPC calls
const tasks = await client.list_tasks();
```

## 🚀 Production Deployment

### Security Checklist
- [ ] Set strong JWT secrets (min 32 chars)
- [ ] Configure CORS for specific origins
- [ ] Enable HTTPS everywhere
- [ ] Set up rate limiting
- [ ] Use environment variables for secrets
- [ ] Enable structured logging
- [ ] Configure database (not JSON files)

### Monitoring Setup
```rust
// Enable OpenTelemetry
let otel = standard_setup("my-service")?;

// Add to service builders
.with_usage_tracker(otel.usage_tracker())
.with_method_duration_tracker(otel.duration_tracker())

// Metrics available at /metrics
```

### Frontend Deployment

#### REST API TypeScript Clients
- Pure JavaScript, no special deployment considerations
- Bundle size ~10KB (vs ~200KB+ for WASM)
- Works in all environments (browsers, Node.js, Deno, etc.)
- Standard JavaScript bundler optimization applies

#### WASM Clients (JSON-RPC)
- Build with `--release` flag
- Use CDN for static WASM assets
- Configure WebSocket proxy for bidirectional services
- Enable gzip/brotli compression
- Consider lazy loading for large WASM modules

## 📖 Additional Resources

- **Detailed Docs**: See `documentation/` directory
- **Examples**: See `examples/` for working code
- **MCP Integration**: Use Context7 for dependency docs
- **Dominator Help**: Ask about reactive UI patterns