# Google OAuth2 Example

A comprehensive example demonstrating Google OAuth2 integration with the Rust Agent Stack identity management system. This example showcases the complete OAuth2 Authorization Code flow with PKCE, JWT session management, and role-based access control through a JSON-RPC API.

## Features

- 🔐 **Secure OAuth2 Flow**: Authorization Code with PKCE for enhanced security
- 🎯 **Role-Based Permissions**: Dynamic permission assignment based on user attributes
- 🚀 **JSON-RPC API**: Type-safe API endpoints with compile-time validation
- ⚡ **JWT Session Management**: Stateless authentication with embedded permissions
- 🛡️ **CSRF Protection**: State parameter validation and secure token handling
- 📚 **Interactive Documentation**: Built-in API documentation and testing interface

## Architecture

This example demonstrates the complete integration of several Rust Agent Stack components:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Frontend      │────│   Axum Server    │────│  JSON-RPC API   │
│   (HTML/JS)     │    │                  │    │   (Macro Gen)   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                               │
                        ┌──────┴──────┐
                        │             │
                ┌───────▼────┐ ┌──────▼──────┐
                │ OAuth2     │ │ Session     │
                │ Provider   │ │ Service     │
                └────────────┘ └─────────────┘
                        │             │
                ┌───────▼────┐ ┌──────▼──────┐
                │ State      │ │ Permissions │
                │ Store      │ │ Provider    │
                └────────────┘ └─────────────┘
```

### Key Components

1. **OAuth2Provider**: Handles Google OAuth2 flow with PKCE
2. **SessionService**: Manages JWT tokens and session lifecycle
3. **GoogleOAuth2Permissions**: Custom permission assignment logic
4. **JSON-RPC Service**: Type-safe API with automatic auth validation
5. **State Management**: In-memory state store for OAuth2 CSRF protection

## Setup Instructions

### 1. Google Cloud Console Setup

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable the Google+ API (or Google People API)
4. Navigate to **Credentials** → **Create Credentials** → **OAuth 2.0 Client ID**
5. Configure the OAuth consent screen
6. Set **Authorized redirect URIs** to include:
   ```
   http://localhost:3000/auth/callback
   ```

### 2. Environment Configuration

1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your Google OAuth2 credentials:
   ```bash
   GOOGLE_CLIENT_ID=your_client_id.apps.googleusercontent.com
   GOOGLE_CLIENT_SECRET=your_client_secret
   REDIRECT_URI=http://localhost:3000/auth/callback
   JWT_SECRET=your-super-secret-jwt-key-change-in-production
   ```

### 3. Run the Application

From the workspace root:

```bash
# Build the example
cargo build -p google-oauth-example

# Run the server
cargo run -p google-oauth-example
```

The server will start on `http://localhost:3000`.

## Usage Guide

### 1. Authentication Flow

1. Navigate to `http://localhost:3000`
2. Click **"Sign in with Google"**
3. Complete OAuth2 authorization with Google
4. You'll be redirected back with a JWT token
5. Use the token to access protected API endpoints

### 2. Testing the API

The application provides several test endpoints demonstrating different permission levels:

#### Basic User Operations
```bash
# Get user information
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "get_user_info",
    "params": {},
    "id": 1
  }'

# List documents
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "list_documents",
    "params": {"limit": 10},
    "id": 2
  }'
```

#### Content Creation (Elevated Permissions)
```bash
# Create document (requires content:create permission)
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_document",
    "params": {
      "title": "My New Document",
      "content": "Document content here...",
      "tags": ["example", "api"]
    },
    "id": 3
  }'
```

#### Admin Operations
```bash
# Delete document (requires admin:write permission)
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "delete_document",
    "params": {"document_id": "doc_123"},
    "id": 4
  }'

# System status (requires system:admin permission)
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "get_system_status",
    "params": {},
    "id": 5
  }'
```

## Permission System

The application demonstrates a sophisticated permission system based on user attributes:

### Permission Assignment Rules

| User Type | Criteria | Permissions |
|-----------|----------|-------------|
| **Basic User** | Any authenticated user | `user:read`, `profile:read` |
| **Verified User** | Email verified by OAuth provider | + `email:verified` |
| **Trusted Domain** | Email ends with `@trusted-domain.com` | + `user:write`, `content:create`, `content:edit` |
| **Admin User** | Email ends with `@example.com` | + `admin:read`, `admin:write`, `system:manage` |
| **System Admin** | Special subject ID | + `system:admin`, `debug:access` |
| **Beta User** | Subject starts with `beta_` | + `beta:access`, `feature:preview` |

### Testing Different Permission Levels

To test different permission levels, you can:

1. **Use different email domains** during OAuth2 login
2. **Modify the permission logic** in `src/permissions.rs`
3. **Create test users** with specific subject IDs

## Security Features

### OAuth2 Security
- **PKCE (Proof Key for Code Exchange)** for enhanced security
- **State parameter validation** to prevent CSRF attacks
- **Secure token exchange** with proper error handling
- **Configurable scopes** for minimal access principle

### JWT Security
- **Configurable JWT secrets** (change in production!)
- **Token expiration** with configurable TTL
- **Session tracking** for token revocation
- **Embedded permissions** for stateless authorization

### API Security
- **Authentication required** for all endpoints
- **Permission-based authorization** for fine-grained access control
- **Input validation** and error handling
- **CORS configuration** for cross-origin requests

## Development

### Project Structure

```
examples/google-oauth-example/
├── src/
│   ├── main.rs              # Main application and server setup
│   ├── permissions.rs       # Custom permission provider
│   └── service.rs          # JSON-RPC service definitions
├── static/
│   ├── index.html          # Frontend interface
│   └── api-docs.html       # API documentation
├── .env.example            # Environment configuration template
├── Cargo.toml              # Dependencies and metadata
└── README.md               # This file
```

### Key Dependencies

- **rust-identity-oauth2**: OAuth2 provider implementation
- **rust-identity-session**: JWT session management
- **rust-jsonrpc-macro**: Type-safe JSON-RPC service generation
- **axum**: Web framework for HTTP handling
- **tower-http**: Middleware for CORS and static files

### Running Tests

```bash
# Run all tests
cargo test -p google-oauth-example

# Run with output
cargo test -p google-oauth-example -- --nocapture

# Run specific test
cargo test -p google-oauth-example test_permissions
```

### Development Tips

1. **Enable debug logging**:
   ```bash
   RUST_LOG=debug cargo run -p google-oauth-example
   ```

2. **Use ngrok for HTTPS testing**:
   ```bash
   ngrok http 3000
   # Update REDIRECT_URI to use the ngrok URL
   ```

3. **Modify permissions** in `src/permissions.rs` to test different access patterns

4. **Add new API endpoints** by extending the service in `src/service.rs`

## Common Issues

### OAuth2 Redirect URI Mismatch
- Ensure the redirect URI in Google Cloud Console matches exactly
- Check for trailing slashes or protocol mismatches
- Verify the REDIRECT_URI environment variable

### JWT Token Issues
- Ensure JWT_SECRET is properly set and consistent
- Check token expiration times
- Verify the Authorization header format: `Bearer TOKEN`

### Permission Denied Errors
- Check user's email domain against permission rules
- Verify the required permissions for the endpoint
- Use `get_user_info` to see current user permissions

### CORS Issues
- The application includes CORS middleware for development
- Modify CORS settings in `main.rs` for production use

## Production Considerations

### Security
- **Change JWT_SECRET** to a strong, random value
- **Use HTTPS** in production (update redirect URIs)
- **Implement rate limiting** for authentication endpoints
- **Add request logging** and monitoring
- **Review permission logic** for your specific use case

### Scalability
- **Replace InMemoryStateStore** with Redis or database
- **Implement session storage** backend (Redis/database)
- **Add connection pooling** for external services
- **Configure proper timeouts** and retry logic

### Monitoring
- **Add health check endpoints**
- **Implement metrics collection**
- **Set up error tracking** and alerting
- **Monitor OAuth2 provider rate limits**

## Related Examples

- **basic-jsonrpc-service**: Simpler JSON-RPC service example
- Check other examples in the `/examples` directory for additional patterns

## Contributing

This example is part of the Rust Agent Stack project. See the main project README for contribution guidelines.

## License

This example follows the same license as the Rust Agent Stack project.