# OpenRPC Types

Complete Rust types for the OpenRPC 1.3.2 specification with serde support, bon builders, and comprehensive validation.

## Features

- **Complete OpenRPC 1.3.2 specification types** - All objects and fields from the specification
- **Serde serialization/deserialization** - Full JSON support with proper field naming
- **Bon builder patterns** - Ergonomic API construction with type-safe builders
- **Comprehensive validation** - Validates against OpenRPC specification constraints
- **JSON Schema Draft 7 compatibility** - For Schema objects with full validation
- **Reference resolution support** - For $ref within components
- **Specification extensions** - Support for x-* extension fields
- **Type safety** - Prevents invalid OpenRPC documents at compile time

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
openrpc-types = "0.1.0"
```

## Example Usage

```rust
use openrpc_types::{OpenRpc, Info, Method, ContentDescriptor, Schema};

// Create an OpenRPC document
let openrpc = OpenRpc::builder()
    .openrpc("1.3.2")
    .info(
        Info::builder()
            .title("Example API")
            .version("1.0.0")
            .description("A simple example API")
            .build()
    )
    .methods(vec![
        Method::builder()
            .name("getUser")
            .params(vec![
                ContentDescriptor::new("userId", Schema::string())
                    .with_description("The user ID")
                    .required()
                    .into()
            ])
            .result(
                ContentDescriptor::new("user", Schema::object()
                    .with_property("id", Schema::string())
                    .with_property("name", Schema::string())
                    .with_property("email", Schema::string())
                ).into()
            )
            .build()
            .into()
    ])
    .build();

// Validate the document
openrpc.validate().expect("Valid OpenRPC document");

// Serialize to JSON
let json = serde_json::to_string_pretty(&openrpc).unwrap();
println!("{}", json);
```

## Core Types

### OpenRPC Document Structure

- **`OpenRpc`** - Root object containing the entire specification
- **`Info`** - Metadata about the API (title, version, description, etc.)
- **`Server`** - Server connectivity information with variable substitution
- **`Method`** - JSON-RPC method definitions with parameters and results
- **`Components`** - Reusable components (schemas, examples, errors, etc.)

### Content and Schema Types

- **`ContentDescriptor`** - Describes parameters and results with schemas
- **`Schema`** - JSON Schema Draft 7 compliant schema definitions
- **`Example`** - Example values with embedded or external references
- **`ExamplePairing`** - Complete request/response example pairs

### Linking and Documentation

- **`Link`** - Runtime links between method results and other methods
- **`Tag`** - Metadata tags for organizing methods
- **`ExternalDocumentation`** - Links to external documentation
- **`ErrorObject`** - Application-defined error specifications

### References and Extensions

- **`Reference`** - Internal and external references using JSON Schema $ref
- **`Extensions`** - Specification extensions with x- prefixed fields

## Validation

The crate provides comprehensive validation that ensures:

- **Specification compliance** - All OpenRPC 1.3.2 constraints are enforced
- **Unique constraints** - Method names, parameter names, error codes are unique
- **Type consistency** - Schemas and examples match their expected types
- **URL and email format validation** - Proper format checking for contact info
- **Parameter ordering** - Required parameters before optional ones
- **Reference validity** - Internal references point to valid components

```rust
use openrpc_types::{OpenRpc, validation::Validate};

let openrpc = // ... build your OpenRPC document

// Validate returns detailed error information
match openrpc.validate() {
    Ok(()) => println!("Valid OpenRPC document"),
    Err(e) => eprintln!("Validation failed: {}", e),
}
```

## JSON Schema Integration

Schema objects are fully compatible with JSON Schema Draft 7:

```rust
use openrpc_types::Schema;

let user_schema = Schema::object()
    .with_property("id", Schema::string().with_format("uuid"))
    .with_property("name", Schema::string().with_min_length(1))
    .with_property("email", Schema::string().with_format("email"))
    .with_property("age", Schema::integer().with_minimum(0.0))
    .require_property("id")
    .require_property("name")
    .require_property("email");
```

## Builder Patterns

All major types support ergonomic builder patterns using the `bon` crate:

```rust
use openrpc_types::{Method, ContentDescriptor, Schema, ParameterStructure};

let method = Method::builder()
    .name("createUser")
    .summary("Create a new user")
    .description("Creates a new user account")
    .params(vec![
        ContentDescriptor::new("userData", Schema::object())
            .with_description("User data")
            .required()
            .into()
    ])
    .result(
        ContentDescriptor::new("userId", Schema::string())
            .with_description("Created user ID")
            .into()
    )
    .param_structure(ParameterStructure::ByName)
    .build();
```

## Error Handling

Comprehensive error types provide detailed information about validation failures:

```rust
use openrpc_types::{OpenRpcError, OpenRpcResult};

fn validate_document(openrpc: &OpenRpc) -> OpenRpcResult<()> {
    openrpc.validate()
}

// Error types include:
// - ValidationError: Specification constraint violations
// - JsonError: JSON parsing/serialization errors  
// - ReferenceError: Invalid or unresolvable references
// - MissingField: Required fields not provided
// - InvalidField: Field values that don't meet constraints
```

## Features

### Optional Features

- **`json-schema`** - Enables additional JSON Schema functionality via `schemars`

```toml
[dependencies]
openrpc-types = { version = "0.1.0", features = ["json-schema"] }
```

## License

This project is licensed under the same terms as the workspace.

## Contributing

This crate is part of the Rust Agent Stack workspace. Please see the main repository for contributing guidelines.