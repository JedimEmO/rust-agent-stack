# Bidirectional Chat Server Integration Tests

This directory contains comprehensive integration tests for the bidirectional chat server. The tests are organized into two main test files:

## Test Files

### `server_tests.rs`
Basic server functionality and configuration tests:
- **Configuration Tests**: Validates default values, configuration parsing, and validation logic
- **Persistence Tests**: Tests the persistence layer for storing and loading chat state
- **Server Lifecycle**: Tests server startup, health checks, and basic endpoints
- **Authentication Endpoints**: Tests user registration and login endpoints
- **Rate Limiting Configuration**: Validates rate limiting settings
- **CORS Configuration**: Tests CORS settings and validation
- **Logging Configuration**: Validates logging settings

### `websocket_tests.rs`
WebSocket and chat-specific functionality tests:
- **Server Lifecycle**: Tests server startup and shutdown
- **User Authentication**: Tests login with valid/invalid credentials
- **User Registration**: Tests new user registration flow
- **Admin Permissions**: Tests admin vs regular user permissions
- **Concurrent Users**: Tests multiple users connecting simultaneously

## Running the Tests

Run all tests:
```bash
cargo test
```

Run only server configuration tests:
```bash
cargo test --test server_tests
```

Run only WebSocket tests:
```bash
cargo test --test websocket_tests
```

Run a specific test:
```bash
cargo test test_user_authentication
```

## Test Coverage

The tests cover the following areas:

1. **Server Startup and Configuration**
   - Configuration file parsing and validation
   - Environment variable overrides
   - Default value handling
   - Invalid configuration detection

2. **User Registration and Login**
   - New user registration
   - Login with credentials
   - Invalid credential handling
   - Concurrent user sessions

3. **WebSocket Connections**
   - Connection establishment
   - Authentication over WebSocket
   - Connection lifecycle management

4. **Chat Operations** (partially tested)
   - Room management
   - Message sending/receiving
   - User profiles
   - Admin operations

5. **Persistence**
   - Message persistence to disk
   - Room state persistence
   - State recovery after restart

6. **Error Cases and Edge Conditions**
   - Invalid configuration values
   - Authentication failures
   - Concurrent access scenarios

## Test Architecture

The tests use a simplified test server implementation that:
- Creates temporary directories for data storage
- Finds available ports automatically
- Provides minimal implementations of chat functionality
- Supports concurrent test execution

## Future Improvements

Areas for additional test coverage:
1. Full WebSocket message flow testing with real client connections
2. Room management operations (join, leave, list)
3. Message broadcasting to multiple users
4. User profile management
5. Admin operations (kick user, broadcast announcement)
6. Rate limiting behavior
7. Connection reconnection and error recovery
8. Performance and load testing