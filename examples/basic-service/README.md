# Basic Service Example

A complete working example of a JSON-RPC service built with the `rust-jsonrpc-macro` crate.

## Overview

This example demonstrates how to build a fully functional JSON-RPC service with:

- ✅ **Authentication** - JWT-style token validation
- ✅ **Authorization** - Permission-based access control
- ✅ **Multiple Methods** - Unauthorized and permission-based endpoints
- ✅ **Axum Integration** - Complete web server setup
- ✅ **Error Handling** - Proper JSON-RPC error responses

## Service Definition

The service implements three methods:

1. **`sign_in`** (UNAUTHORIZED) - Authenticate users and return JWT tokens
2. **`sign_out`** (requires valid token) - Sign out authenticated users  
3. **`delete_everything`** (requires "admin" permission) - Admin-only destructive operation

## Running the Example

```bash
# From the workspace root
cargo run -p basic-service
```

The server will start on `http://0.0.0.0:3000`

## Testing the Service

### 1. Sign In (Get a Token)

**Admin User:**
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_in",
    "params": {
      "WithCredentials": {
        "username": "admin",
        "password": "secret"
      }
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "Success": {
      "jwt": "admin_token"
    }
  },
  "id": 1
}
```

**Regular User:**
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_in",
    "params": {
      "WithCredentials": {
        "username": "user",
        "password": "password"
      }
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "Success": {
      "jwt": "valid_token"
    }
  },
  "id": 1
}
```

### 2. Sign Out (Requires Authentication)

```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer valid_token" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_out",
    "params": {},
    "id": 2
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": null,
  "id": 2
}
```

### 3. Delete Everything (Requires Admin Permission)

**With Admin Token:**
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer admin_token" \
  -d '{
    "jsonrpc": "2.0",
    "method": "delete_everything",
    "params": {},
    "id": 3
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": null,
  "id": 3
}
```

**With Regular User Token (Insufficient Permissions):**
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer valid_token" \
  -d '{
    "jsonrpc": "2.0",
    "method": "delete_everything",
    "params": {},
    "id": 3
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32002,
    "message": "Insufficient permissions",
    "data": {
      "required": ["admin"],
      "has": ["user"]
    }
  },
  "id": 3
}
```

## Error Cases

### Invalid Credentials
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_in",
    "params": {
      "WithCredentials": {
        "username": "wrong",
        "password": "wrong"
      }
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "Failure": {
      "msg": "Invalid credentials"
    }
  },
  "id": 1
}
```

### Missing Authentication
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_out",
    "params": {},
    "id": 2
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Authentication required"
  },
  "id": 2
}
```

### Invalid Token
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer invalid_token" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_out",
    "params": {},
    "id": 2
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Authentication required"
  },
  "id": 2
}
```

### Method Not Found
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "unknown_method",
    "params": {},
    "id": 4
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found: unknown_method"
  },
  "id": 4
}
```

## Code Structure

### Authentication Provider

The example implements a simple `MyAuthProvider` that:

- Validates tokens (`"valid_token"` for users, `"admin_token"` for admins)
- Assigns permissions based on token type
- Returns `AuthenticatedUser` with user ID and permissions

### Service Implementation

The service is defined using the `jsonrpc_service!` macro and configured with:

- **Base URL**: `/rpc` (nested under `/api` in the router)
- **Auth Provider**: `MyAuthProvider` instance
- **Method Handlers**: Async closures for each defined method

### Integration

The service integrates with axum as a standard router that can be:

- Nested under other routes
- Combined with additional middleware
- Composed with other axum services

## Key Features Demonstrated

1. **Type Safety**: All request/response types are validated at compile time
2. **Authentication Flow**: Token extraction, validation, and error handling
3. **Permission Checking**: Fine-grained authorization with clear error messages
4. **JSON-RPC Compliance**: Proper error codes and response formats
5. **Async Support**: All handlers use async/await for non-blocking operations
6. **Logging**: Structured logging with the `tracing` crate

## Extending the Example

To add new methods:

1. **Define Request/Response Types**:
   ```rust
   #[derive(Serialize, Deserialize)]
   struct CreateUserRequest {
       username: String,
       email: String,
   }
   ```

2. **Add to Service Definition**:
   ```rust
   jsonrpc_service!({
       service_name: MyService,
       methods: [
           // ... existing methods
           WITH_PERMISSIONS(["admin"]) create_user(CreateUserRequest) -> UserId,
       ]
   });
   ```

3. **Implement Handler**:
   ```rust
   .create_user_handler(|user, request| async move {
       // Implementation here
       Ok(UserId { id: "new_user_123".to_string() })
   })
   ```

## Dependencies

See [`Cargo.toml`](Cargo.toml) for the complete dependency list. Key dependencies include:

- `rust-jsonrpc-macro` - The procedural macro
- `rust-jsonrpc-core` - Authentication traits
- `axum` - Web framework
- `tokio` - Async runtime
- `serde` - Serialization
- `tracing` - Logging