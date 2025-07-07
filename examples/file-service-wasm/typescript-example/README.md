# File Service SolidJS App

A modern file upload/download service built with SolidJS, TypeScript, and WebAssembly (Rust).

## Features

- 🚀 **SolidJS** - Reactive UI framework with fine-grained reactivity
- 🦀 **Rust + WebAssembly** - High-performance client using the `file_service!` macro
- 🔐 **Authentication** - JWT token support with secure/public endpoints
- 📁 **File Operations** - Drag-and-drop upload, download with progress
- 💅 **Modern UI** - Tailwind CSS with responsive design
- ⚡ **Vite** - Fast development and optimized production builds

## Prerequisites

Build the WASM package first:
```bash
cd ../file-service-api
./build-wasm.sh
```

## Development

1. Install dependencies:
   ```bash
   npm install
   ```

2. Start the development server:
   ```bash
   npm run dev
   ```

3. Open http://localhost:3001 in your browser

## Building for Production

```bash
npm run build
npm run preview
```

## Architecture

### Components

- **FileUpload** - Drag-and-drop file upload with progress
- **FileList** - Display uploaded files with download functionality
- **AuthForm** - JWT token management

### State Management

- **auth.ts** - Authentication state with localStorage persistence
- **client.ts** - WASM client initialization and management

### API Integration

The app uses the generated WASM client from the `file_service!` macro:

```typescript
import init, { WasmDocumentServiceClient } from '/pkg/file_service_api.js';

// Initialize once
await init();

// Create client
const client = new WasmDocumentServiceClient(window.location.origin);

// Use the client
const response = await client.upload(file);
```

## Configuration

The Vite dev server proxies API requests to `http://localhost:3000`. Update `vite.config.ts` if your backend runs on a different port.

## Security

- Tokens are stored in localStorage
- WASM client handles authentication headers automatically
- Secure endpoints require valid JWT tokens

## Browser Support

Requires browsers with WebAssembly support (all modern browsers).