# REST API WASM Client Example

This example demonstrates how to use the `ras-rest-macro` to generate TypeScript/WASM clients for Rust REST APIs.

## Project Structure

- `rest-api/` - Shared API definitions using `rest_service!` macro
- `rest-backend/` - Backend server implementation
- `typescript-example/` - TypeScript web app using the WASM client

## Features

- **Type-safe API definitions** - Single source of truth in Rust
- **Automatic WASM client generation** - TypeScript bindings via wasm-bindgen
- **Authentication support** - Built-in bearer token handling
- **Cross-platform** - Same macro generates native and WASM clients

## Quick Start

### 1. Start Backend Server

```bash
cd rest-backend
cargo run
```

The server will run at http://localhost:3000 with:
- API endpoints at `/api/v1/*`
- OpenAPI docs at `/api/v1/docs`

### 2. Start TypeScript App

```bash
cd typescript-example
npm install
npm run dev
```

Open http://localhost:3001 in your browser.

The Vite development server will automatically:
- Build the WASM package from the Rust source
- Watch for changes and rebuild when needed
- Serve the app with hot module replacement

## API Endpoints

### Public Endpoints (No Auth Required)
- `GET /api/v1/users` - Get all users
- `GET /api/v1/users/{id}` - Get user by ID

### Protected Endpoints (Auth Required)
- `POST /api/v1/users` - Create user (admin only)
- `PUT /api/v1/users/{id}` - Update user (admin only)
- `DELETE /api/v1/users/{id}` - Delete user (admin only)
- `GET /api/v1/users/{user_id}/tasks` - Get user tasks
- `POST /api/v1/users/{user_id}/tasks` - Create task
- `PUT /api/v1/users/{user_id}/tasks/{task_id}` - Update task
- `DELETE /api/v1/users/{user_id}/tasks/{task_id}` - Delete task

## Authentication

The example uses a simple mock authentication for demonstration purposes:

- Use `"validtoken"` as the bearer token for user permissions
- Use `"admintoken"` as the bearer token for admin permissions

In the TypeScript client:
```typescript
// For user access
client.set_bearer_token("validtoken");

// For admin access
client.set_bearer_token("admintoken");
```

## TypeScript Client Usage

```typescript
import init, { WasmUserServiceClient } from '@wasm/rest_api.js';

// Initialize WASM module
await init();

// Create client
const client = new WasmUserServiceClient('http://localhost:3000');

// Set authentication token
client.set_bearer_token('your-jwt-token');

// Make API calls
const users = await client.get_users();
const user = await client.get_users_by_id('1');

// Create user (requires admin role)
const newUser = await client.post_users({
  name: 'John Doe',
  email: 'john@example.com'
});
```

## How It Works

1. The `rest_service!` macro in `rest-api/src/lib.rs` defines the API
2. The macro generates:
   - Native Rust client (`UserServiceClient`)
   - WASM wrapper client (`WasmUserServiceClient`) when `wasm-client` feature is enabled
   - Server trait and builder (`UserServiceTrait`, `UserServiceBuilder`)
3. The Vite plugin automatically runs `wasm-pack` to build the WASM module
4. The TypeScript app imports and uses the generated client with full type safety

## Benefits

- **Type Safety**: API changes are caught at compile time
- **No Manual Client Maintenance**: Client code is auto-generated
- **Performance**: Direct WASM execution for data processing
- **Developer Experience**: Full IDE support with TypeScript types