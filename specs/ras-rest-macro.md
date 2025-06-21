# ras-rest-macro Security and Design Analysis

## Executive Summary

The `ras-rest-macro` crate provides a procedural macro for generating type-safe REST APIs with authentication integration and OpenAPI 3.0 documentation. While the crate demonstrates good architectural patterns and type safety, this analysis has identified several critical security vulnerabilities and design limitations that must be addressed before production use.

## Critical Security Vulnerabilities

### 1. Path Traversal in Static File Serving (CRITICAL)

**Location**: `src/static_hosting.rs:1185-1189`

The `docs_path` parameter is directly interpolated into routes without validation:
```rust
router = router
    .route(#docs_path, ::axum::routing::get(#docs_handler_name))
    .route(&format!("{}/openapi.json", #docs_path), ::axum::routing::get(openapi_json_handler));
```

**Risk**: An attacker could potentially access sensitive files by providing a malicious `docs_path` like `"/../../../etc/passwd"`.

**Recommendation**: Validate and sanitize the `docs_path` parameter, ensuring it only contains safe characters and doesn't contain path traversal sequences.

### 2. Cross-Site Scripting (XSS) in Documentation UI (CRITICAL)

**Location**: `src/static_hosting.rs:57`

The OpenAPI specification is embedded directly into HTML without proper escaping:
```rust
let spec_json = ::serde_json::to_string_pretty(&openapi_spec)
    .unwrap_or_else(|_| "{}".to_string());
```

**Risk**: If the OpenAPI spec contains malicious JavaScript strings (e.g., in descriptions), they could be executed in the browser.

**Recommendation**: HTML-escape the JSON before embedding it in the page, or load it via a separate AJAX request.

## High Severity Issues

### 1. Information Disclosure via Error Messages

**Location**: `src/lib.rs:735-740, 869-875`

Internal error messages are exposed directly to clients:
```rust
Err(e) => {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(serde_json::json!({
            "error": e.to_string()  // Full error details exposed
        }))
    ).into_response()
}
```

**Risk**: Stack traces, file paths, database schemas, and other sensitive information could be leaked to attackers.

**Recommendation**: Log detailed errors server-side but return generic error messages to clients.

### 2. Missing Input Validation

The macro generates no input validation for:
- Path parameters
- Query parameters
- Request bodies

**Risk**: Invalid input causes 500 errors instead of proper 400 Bad Request responses, and could lead to injection attacks.

**Recommendation**: Add validation attributes and generate validation code for all inputs.

### 3. Insecure JWT Storage

**Location**: `src/static_hosting.rs:765-774`

JWT tokens are stored in localStorage:
```javascript
localStorage.setItem('jwt-token', jwtToken);
```

**Risk**: Tokens are vulnerable to XSS attacks. Any JavaScript can read localStorage.

**Recommendation**: Use httpOnly cookies or at minimum sessionStorage with additional XSS protections.

## Medium Severity Issues

### 1. Missing CORS Support

The generated services have no CORS handling, preventing browser-based clients from different origins.

**Recommendation**: Add configurable CORS middleware generation.

### 2. No Rate Limiting

Services are vulnerable to DoS attacks without rate limiting.

**Recommendation**: Add optional rate limiting middleware generation.

### 3. Inefficient Permission Checking

**Location**: `src/lib.rs:831-858`

Permission checking uses sequential iteration and has a potential panic:
```rust
auth_provider.as_ref().unwrap().check_permissions(&user, permission_group);
```

**Recommendation**: Optimize permission checking and handle the None case properly.

### 4. Limited Content-Type Support

The macro assumes all requests/responses are JSON, preventing file uploads or other content types.

**Recommendation**: Add content-type negotiation support.

## Design Limitations

### 1. Framework Coupling

The macro is tightly coupled to Axum and cannot be used with other web frameworks.

**Impact**: Limits adoption and flexibility.

### 2. Limited HTTP Method Support

Only supports GET, POST, PUT, DELETE, PATCH. Missing HEAD, OPTIONS, and custom methods.

### 3. No Middleware Support

Cannot add custom middleware to specific endpoints or services.

### 4. Static Documentation

OpenAPI docs are generated at compile-time and cannot reflect runtime changes.

## Code Quality Issues

### 1. Poor Error Spans

Many errors use `input.span()` providing poor error location information.

**Recommendation**: Use more specific spans for better error messages.

### 2. String-Based Path Parsing

**Location**: `src/lib.rs:327-362`

Path parsing uses fragile string manipulation instead of a proper parser.

**Recommendation**: Implement a proper path parser or use an existing library.

### 3. Code Duplication

Significant duplication between authenticated and unauthenticated handler generation.

**Recommendation**: Extract common code into helper functions.

### 4. Magic Strings

Important strings like "Authorization" and "Bearer " are hardcoded throughout.

**Recommendation**: Define constants for reusable strings.

## Missing Security Features

1. **No CSRF Protection**: Services are vulnerable to CSRF attacks
2. **No Security Headers**: Missing headers like X-Content-Type-Options, X-Frame-Options
3. **No Request Size Limits**: Vulnerable to large payload attacks
4. **No Timeout Configuration**: Cannot configure per-endpoint timeouts

## Performance Considerations

### Positive
- No synchronous blocking in async contexts
- Efficient macro expansion

### Negative
- Multiple unnecessary clones in hot paths
- Client creates new connections for each request (no pooling)
- Runtime string allocations for permission checking

## Recommendations

### Immediate Actions (Before Release)

~~1. **Fix Path Traversal**: Validate and sanitize all path inputs~~
~~2. **Fix XSS**: Properly escape OpenAPI spec in HTML~~
~~3. **Sanitize Errors**: Return generic errors to clients, log details server-side~~
4. **Add Input Validation**: Generate validation code for all inputs

### Short-Term Improvements

1. **Add CORS Support**: Configurable CORS middleware
2. **Add Logging**: Request/response logging with correlation IDs
3. **Improve Errors**: Consistent error response format
4. **Add Middleware**: Support for custom middleware

### Long-Term Enhancements

1. **Framework Agnostic**: Abstract away from Axum
2. **OpenAPI 3.1**: Support latest specification
3. **Rate Limiting**: Built-in rate limiting
4. **Metrics & Tracing**: OpenTelemetry integration

## Testing Gaps

The integration tests cover basic functionality but miss:
- Security scenarios (injection, traversal, XSS)
- Error handling edge cases
- Performance under load
- Concurrent request handling

## Conclusion

The `ras-rest-macro` crate shows promise with its type-safe approach and OpenAPI integration, but it requires significant security hardening before production use. The critical vulnerabilities (path traversal and XSS) must be fixed immediately. The design limitations around framework coupling and limited HTTP support may require architectural changes to address.

Priority should be given to:
1. Fixing security vulnerabilities
2. Adding input validation
3. Improving error handling
4. Adding essential features (CORS, logging)

With these improvements, the crate could provide a solid foundation for building secure, well-documented REST APIs in Rust.