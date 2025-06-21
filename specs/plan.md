# JSON-RPC API Explorer Implementation Plan

## Overview
Create an interactive API explorer for JSON-RPC services using OpenRPC documents, similar to the REST API explorer in `ras-rest-macro`.

## Step-by-Step Implementation Plan

### Phase 1: Create Static Hosting Module
1. **Create `static_hosting.rs` module** in `crates/libs/ras-jsonrpc-macro/src/`
   - Define `StaticHostingConfig` struct with options:
     - `serve_explorer`: bool (enable/disable)
     - `explorer_path`: String (default "/explorer")
     - `ui_theme`: String (default "default")
   - Implement `generate_static_hosting_code()` function
   - Implement `generate_static_routes()` function

### Phase 2: Update Macro Configuration
2. **Update `lib.rs`** to support static hosting
   - Add `static_hosting` module
   - Update `ServiceDefinition` to include explorer config
   - Parse explorer options from macro input (e.g., `explorer: true` or `explorer: { path: "/docs" }`)
   - Generate static hosting code when explorer is enabled

### Phase 3: Create JSON-RPC Explorer UI
3. **Design Explorer HTML/CSS/JS**
   - Adapt the REST explorer UI for JSON-RPC:
     - Replace "endpoints" with "methods"
     - Remove HTTP method badges (GET, POST, etc.)
     - Add JSON-RPC specific UI elements (request ID, jsonrpc version)
   - Key UI Components:
     - **Sidebar**: Method list with search, authentication section
     - **Main Panel**: Method details, request builder, response viewer
     - **Request Builder**: 
       - Auto-generate request ID
       - Fixed jsonrpc: "2.0"
       - Method name (auto-filled)
       - Params editor with schema-based examples
     - **Response Viewer**: Format JSON-RPC responses with error handling

### Phase 4: OpenRPC Integration
4. **Parse and Use OpenRPC Document**
   - JavaScript functions to:
     - Load OpenRPC document from `/explorer/openrpc.json`
     - Extract method information
     - Generate parameter forms from schemas
     - Handle authentication metadata (`x-authentication`, `x-permissions`)
   - Schema handling:
     - Generate example JSON from schemas
     - Validate parameters against schemas
     - Show parameter descriptions and types

### Phase 5: JSON-RPC Request/Response Handling
5. **Implement JSON-RPC Communication**
   - Build proper JSON-RPC requests:
     ```json
     {
       "jsonrpc": "2.0",
       "method": "method_name",
       "params": { ... },
       "id": "unique-id"
     }
     ```
   - Handle JSON-RPC responses and errors
   - Display error codes and messages appropriately
   - Support batch requests (optional, phase 2)

### Phase 6: Authentication Integration
6. **JWT Authentication Support**
   - Store JWT tokens in localStorage
   - Add Authorization header to requests
   - Visual indicators for authenticated state
   - Show required permissions for each method

### Phase 7: Route Generation
7. **Generate Axum Routes**
   - `/explorer` - Main explorer HTML page
   - `/explorer/openrpc.json` - OpenRPC document endpoint
   - Integrate with existing router in macro expansion

### Phase 8: Testing and Polish
8. **Create Tests and Examples**
   - Integration test with a sample JSON-RPC service
   - Update existing examples to show explorer usage
   - Test with various parameter types and authentication scenarios
   - Ensure XSS protection in generated HTML
   - Add proper error handling and edge cases

## Technical Considerations

- **Single Endpoint**: Unlike REST, all JSON-RPC requests go to the same endpoint (base path)
- **Request Format**: Must follow JSON-RPC 2.0 specification exactly
- **Error Handling**: Map JSON-RPC error codes to user-friendly messages
- **Schema Compatibility**: Ensure generated schemas work with the explorer's form generation
- **Performance**: Consider lazy loading for services with many methods
- **Security**: Sanitize all user inputs, especially in generated HTML

## Implementation Order
1. Static hosting module structure
2. Basic HTML page generation
3. OpenRPC loading and parsing
4. Method list and search
5. Request builder with examples
6. JSON-RPC request sending
7. Response display
8. Authentication features
9. Polish and testing

## Success Criteria
- Explorer loads and displays all methods from OpenRPC
- Can send valid JSON-RPC requests to all methods
- Properly handles authentication and permissions
- Generates correct parameter examples from schemas
- Clean, intuitive UI similar to REST explorer
- No security vulnerabilities (XSS, injection)