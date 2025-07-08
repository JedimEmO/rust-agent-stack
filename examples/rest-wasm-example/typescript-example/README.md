# REST API TypeScript Example

This example demonstrates using the WASM-generated TypeScript client for the REST API.

## Quick Start

### 1. Start the Backend Server

```bash
cd ../rest-backend
cargo run
```

The server will run on http://localhost:3000 with API docs at http://localhost:3000/api/v1/docs.

### 2. Start the TypeScript App

```bash
npm install
npm run dev
```

The app will be available at http://localhost:3001.

## How It Works

The Vite development server automatically:
1. Builds the WASM package from the Rust source using `wasm-pack`
2. Watches for changes in the Rust code and rebuilds automatically
3. Serves the TypeScript app with hot module replacement

No manual WASM building or copying is required!

## Features

- **Automatic WASM Building**: The `vite-plugin-wasm-pack` plugin handles building the WASM package
- **Type-Safe API Calls**: Full TypeScript type definitions generated from Rust
- **Authentication**: Simple token-based auth with convenient UI buttons
- **Real-time Updates**: Changes to Rust code trigger automatic rebuilds

## Authentication

The example uses simple mock authentication:
- Click "Set User Token" or use `"validtoken"` for user permissions
- Click "Set Admin Token" or use `"admintoken"` for admin permissions

## API Endpoints

### Public Endpoints (No Auth Required)
- `GET /api/v1/users` - Get all users
- `GET /api/v1/users/{id}` - Get user by ID

### Protected Endpoints (Auth Required)
- `POST /api/v1/users` - Create user (admin only)
- `PUT /api/v1/users/{id}` - Update user (admin only)
- `DELETE /api/v1/users/{id}` - Delete user (admin only)
- Task management endpoints (user role)

## Technologies

- SolidJS for the reactive UI
- Vite for development and bundling
- WebAssembly for the Rust client
- TypeScript for type safety