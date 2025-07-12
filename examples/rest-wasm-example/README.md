# REST API TypeScript Client Example

This example demonstrates how to use the `ras-rest-macro` to generate OpenAPI specifications at compile time, which are then used to generate type-safe TypeScript clients.

## Project Structure

- `rest-api/` - Shared API definitions using `rest_service!` macro
- `rest-backend/` - Backend server implementation with OpenAPI generation
- `typescript-example/` - TypeScript web app using the generated TypeScript client

## Features

- **Type-safe API definitions** - Single source of truth in Rust
- **Automatic API client generation** - TypeScript client generated from OpenAPI spec via openapi-ts
- **Authentication support** - Built-in bearer token handling
- **Zero manual types** - All types are auto-generated from Rust definitions

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
- Watch for changes to the OpenAPI specification
- Regenerate the TypeScript client when the spec changes
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
import * as api from './generated/services.gen';

// For user access
const response = await api.getUsersUserIdTasks({
  baseUrl: 'http://localhost:3000/api/v1',
  headers: {
    Authorization: 'Bearer validtoken'
  },
  path: { user_id: '123' }
});

// For admin access
const response = await api.postUsers({
  baseUrl: 'http://localhost:3000/api/v1',
  headers: {
    Authorization: 'Bearer admintoken'
  },
  body: { name: 'New User', email: 'user@example.com' }
});
```

## TypeScript Client Usage

```typescript
import * as api from './generated/services.gen';
import type { User, CreateUserRequest } from './generated/types.gen';

// Make API calls with full type safety and named methods
const response = await api.getUsers({
  baseUrl: 'http://localhost:3000/api/v1',
});

if (response.data) {
  // response.data is fully typed as UsersResponse
  const users = response.data.users;
}

// Get a specific user
const userResponse = await api.getUsersId({
  baseUrl: 'http://localhost:3000/api/v1',
  path: { id: '1' }
});

// Create user (requires admin role)
const newUser: CreateUserRequest = {
  name: 'John Doe',
  email: 'john@example.com'
};

const createResponse = await api.postUsers({
  baseUrl: 'http://localhost:3000/api/v1',
  headers: {
    Authorization: 'Bearer admintoken'
  },
  body: newUser  // Type-checked!
});

// Get user tasks (requires user role)
const tasksResponse = await api.getUsersUserIdTasks({
  baseUrl: 'http://localhost:3000/api/v1',
  headers: {
    Authorization: 'Bearer validtoken'
  },
  path: { user_id: '123' }
});
```

## How It Works

1. The `rest_service!` macro in `rest-api/src/lib.rs` defines the API
2. The macro generates:
   - Native Rust client (`UserServiceClient`)
   - Server trait and builder (`UserServiceTrait`, `UserServiceBuilder`)
   - OpenAPI specification generation function
3. The backend's `build.rs` generates the OpenAPI spec at compile time
4. A Vite plugin watches the OpenAPI spec and regenerates the TypeScript client
5. The TypeScript app imports and uses the generated client with full type safety

## Benefits

- **Type Safety**: API changes are caught at compile time
- **No Manual Client Maintenance**: Client code is auto-generated from OpenAPI spec
- **Smaller Bundle Size**: Pure TypeScript, no WASM overhead (~10KB vs ~200KB+)
- **Better Compatibility**: Works in all JavaScript environments (Node.js, browsers, etc.)
- **Developer Experience**: Full IDE support with TypeScript types and auto-completion
- **Simpler Build Process**: No need for wasm-pack or WASM toolchain
