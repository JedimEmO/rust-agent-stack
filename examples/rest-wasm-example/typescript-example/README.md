# TypeScript REST API Client Example

This example demonstrates how to generate a fully type-safe TypeScript client from an OpenAPI specification that's generated at compile time from Rust.

## Features

- 🚀 **Automatic Client Generation**: TypeScript client is generated from OpenAPI spec
- 🔄 **Hot Reload**: Client regenerates when OpenAPI spec changes
- 📘 **Full Type Safety**: Complete type inference for requests and responses
- 🛡️ **Type-Safe Errors**: Typed error responses
- 🔧 **Zero Manual Types**: No need to maintain TypeScript type definitions

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌────────────────┐
│   Rust API      │────>│  OpenAPI Spec    │────>│  TS Client     │
│   Definition    │     │  (compile time)  │     │  (generated)   │
└─────────────────┘     └──────────────────┘     └────────────────┘
        │                                                  │
        └──────────────────────────────────────────────────┘
                    Full end-to-end type safety
```

## How it Works

1. **Compile Time OpenAPI Generation**: When building the Rust backend, the OpenAPI spec is generated at compile time via `build.rs`
2. **Vite Plugin**: A custom Vite plugin watches for changes to the OpenAPI spec
3. **Automatic Generation**: When the spec changes, `openapi-ts` generates a new TypeScript client
4. **Type Safety**: The generated client provides full type safety with auto-completion

## Setup

```bash
# Install dependencies
npm install

# Build the Rust backend first (generates OpenAPI spec)
cd ../rest-backend
cargo build

# Run the TypeScript dev server
cd ../typescript-example
npm run dev
```

The app will be available at http://localhost:3001.

## Usage

The generated client provides named methods with full type safety:

```typescript
import * as api from './generated/services.gen';
import type { User, CreateUserRequest } from './generated/types.gen';

// Make type-safe API calls with named methods
const response = await api.getUsers({
  baseUrl: 'http://localhost:3000/api/v1',
});

if (response.data) {
  // response.data is fully typed as UsersResponse
  console.log(response.data.users);
}

// Get a specific user by ID
const userResponse = await api.getUsersId({
  baseUrl: 'http://localhost:3000/api/v1',
  path: { id: '123' }
});

// Create a user with type checking (requires admin auth)
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
```

## Generated Files

- `src/generated/types.gen.ts` - All TypeScript types from OpenAPI schemas
- `src/generated/services.gen.ts` - The API client with all endpoints

## Authentication

The example backend uses simple mock authentication:
- Use `"validtoken"` for user permissions
- Use `"admintoken"` for admin permissions

Authentication is passed in the method call:
```typescript
const response = await api.getUsersUserIdTasks({
  baseUrl: 'http://localhost:3000/api/v1',
  headers: {
    Authorization: 'Bearer validtoken'
  },
  path: { user_id: '123' }
});
```

## API Endpoints

### Public Endpoints (No Auth Required)
- `GET /api/v1/users` - Get all users
- `GET /api/v1/users/{id}` - Get user by ID

### Protected Endpoints (Auth Required)
- `POST /api/v1/users` - Create user (admin only)
- `PUT /api/v1/users/{id}` - Update user (admin only)
- `DELETE /api/v1/users/{id}` - Delete user (admin only)
- `GET /api/v1/users/{user_id}/tasks` - Get user tasks (user role)
- `POST /api/v1/users/{user_id}/tasks` - Create task (user role)
- `PUT /api/v1/users/{user_id}/tasks/{task_id}` - Update task (user role)
- `DELETE /api/v1/users/{user_id}/tasks/{task_id}` - Delete task (user role)

## Configuration

Edit `openapi-ts.config.ts` to customize the client generation:

```typescript
export default defineConfig({
  client: '@hey-api/client-fetch',
  input: '../rest-backend/target/openapi/userservice.json',
  output: {
    path: './src/generated',
    format: 'prettier',
  },
  types: {
    enums: 'javascript',
  },
  services: {
    asClass: true,
  },
});
```

## Benefits

1. **No Manual Sync**: Types are always in sync with the Rust API
2. **Catch Errors Early**: TypeScript compiler catches API mismatches
3. **Great DX**: Auto-completion and inline documentation
4. **Maintainable**: Single source of truth in Rust
5. **Smaller Bundle**: No WASM overhead, pure TypeScript

## Comparison with WASM Client

### Before (WASM Client)
- Required WASM compilation with wasm-pack
- Manual type definitions needed
- Complex build process
- Larger bundle size (~200KB+ for WASM)
- Platform-specific limitations

### After (OpenAPI Client)  
- Pure TypeScript/JavaScript
- Automatic type generation from OpenAPI
- Simple build process
- Smaller bundle size (~10KB)
- Works everywhere (Node.js, browsers, etc.)

## Technologies

- **SolidJS** - Reactive UI framework
- **Vite** - Fast build tool with HMR
- **openapi-ts** - OpenAPI to TypeScript generator
- **TypeScript** - Type safety