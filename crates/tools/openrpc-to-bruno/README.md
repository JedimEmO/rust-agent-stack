# OpenRPC to Bruno Converter

A CLI tool that converts OpenRPC specifications to Bruno API collections, enabling you to quickly create API testing collections from your OpenRPC documentation.

## Features

- ✅ **Complete OpenRPC Support**: Parses OpenRPC 1.3.2 specification files
- ✅ **Bruno Collection Generation**: Creates valid Bruno collection structures
- ✅ **JSON-RPC Request Generation**: Generates proper JSON-RPC 2.0 requests for each method
- ✅ **Environment Variables**: Creates environment files with configurable base URLs
- ✅ **Example Parameter Generation**: Automatically generates example parameters based on schema types
- ✅ **Custom Collection Names**: Support for custom collection names
- ✅ **Comprehensive CLI**: Full command-line interface with verbose output
- ✅ **Authentication Awareness**: Detects authentication requirements from OpenRPC extensions
- ✅ **Workspace Integration**: Fully integrated with the Rust Agent Stack workspace

## Installation

From the workspace root:

```bash
cargo build -p openrpc-to-bruno
```

## Usage

### Basic Usage

```bash
# Convert an OpenRPC file to Bruno collection
cargo run -p openrpc-to-bruno -- \
  --input api.json \
  --output ./bruno-collection

# With custom settings
cargo run -p openrpc-to-bruno -- \
  --input api.json \
  --output ./my-api \
  --base-url https://api.example.com \
  --name "My API Collection" \
  --verbose \
  --force
```

### CLI Options

- `-i, --input <FILE>`: Path to the OpenRPC specification file (JSON)
- `-o, --output <DIR>`: Output directory for the Bruno collection
- `-b, --base-url <URL>`: Base URL for the API server (optional)
- `-n, --name <NAME>`: Collection name (defaults to OpenRPC info.title)
- `-f, --force`: Force overwrite existing Bruno collection
- `-v, --verbose`: Enable verbose output

## Generated Structure

The tool generates a complete Bruno collection:

```
output-directory/
├── bruno.json              # Collection metadata
├── environments/
│   └── default.bru         # Environment variables
├── method1.bru             # Generated request for method1
├── method2.bru             # Generated request for method2
└── ...
```

### Example Generated Files

**bruno.json**:
```json
{
  "version": "1",
  "name": "My API Collection",
  "type": "collection",
  "ignore": ["node_modules", ".git"]
}
```

**environments/default.bru**:
```
vars {
  base_url: https://api.example.com
  api_path: /api/v1/rpc
}
```

**hello.bru**:
```
meta {
  name: hello
  type: http
  seq: 1
}

post {
  url: {{base_url}}{{api_path}}
  body: json
}

headers {
  Content-Type: application/json
}

body:json {
  {
    "id": 1,
    "jsonrpc": "2.0",
    "method": "hello",
    "params": {
      "name": "example_string"
    }
  }
}
```

## Supported OpenRPC Features

- ✅ Basic method definitions
- ✅ Parameter schemas (string, number, integer, boolean, array, object)
- ✅ Result schemas
- ✅ Method descriptions and summaries
- ✅ Server definitions
- ✅ OpenRPC 1.3.2 specification
- ⚠️ Extensions (partial support - x-authentication detection)
- ❌ Reference objects (not yet supported)
- ❌ YAML input files (not yet supported)
- ❌ Complex schema validation (not yet supported)

## Authentication Support

The tool detects authentication requirements from OpenRPC extensions:

- Methods with `x-authentication: true` will include Bearer token authentication
- Methods with `x-permissions` will include Bearer token authentication
- Authentication tokens use `{{auth_token}}` variable placeholder

## Testing

Run the test suite:

```bash
# Run all tests
cargo test -p openrpc-to-bruno

# Run with verbose output
cargo test -p openrpc-to-bruno -- --nocapture
```

Test fixtures are available in `tests/fixtures/` for testing various OpenRPC scenarios.

## Integration

This tool is part of the Rust Agent Stack and integrates with:

- **openrpc-types**: Uses the workspace's OpenRPC type definitions
- **Workspace dependencies**: Shares common dependencies like clap, serde, tokio
- **Error handling**: Uses thiserror for structured error handling
- **Testing**: Comprehensive test coverage with integration tests

## Future Enhancements

- YAML input file support
- Reference object resolution
- More sophisticated schema-to-example generation
- Advanced authentication scheme support
- Custom template support
- Bulk conversion support

## Contributing

This tool follows the workspace's development guidelines. See the main workspace README and CLAUDE.md for development practices and testing requirements.