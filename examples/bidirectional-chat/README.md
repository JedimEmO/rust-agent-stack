# Bidirectional Chat Example

This example demonstrates a real-time chat application using bidirectional JSON-RPC over WebSockets. It showcases:

- **Bidirectional Communication**: Both client and server can initiate messages
- **JWT Authentication**: Secure user authentication with role-based permissions
- **Real-time Updates**: Instant message delivery and presence notifications
- **Room Management**: Multiple chat rooms with user presence tracking
- **Interactive CLI**: User-friendly terminal interface

## Architecture

The example consists of three crates:

1. **bidirectional-chat-api**: Shared types and data structures
2. **bidirectional-chat-server**: WebSocket server with room management
3. **bidirectional-chat-client**: Interactive terminal client

## Features

### Server Features
- Multi-room chat support
- User authentication with local identity provider
- Role-based permissions (user, moderator, admin)
- Real-time message broadcasting
- User presence tracking
- Kick/ban functionality for moderators
- System-wide announcements for admins
- Automatic cleanup on disconnect

### Client Features
- Interactive terminal UI
- Real-time message display
- Room navigation commands
- Colored output for better readability
- Cross-platform WebSocket support (native + WASM)

## Quick Start

### 1. Start the Server

```bash
cargo run -p bidirectional-chat-server
```

The server will start on `http://localhost:3000` with WebSocket endpoint at `ws://localhost:3000/ws`.

### 2. Register Users

Register a new user:
```bash
cargo run -p bidirectional-chat-client register --username alice
# Enter password when prompted
```

Pre-configured users:
- `admin` / `admin123` - Full admin privileges
- `moderator` / `mod123` - Moderator privileges
- `alice` / `alice123` - Regular user
- `bob` / `bob123` - Regular user

### 3. Start Chatting

Login and start the interactive chat:
```bash
cargo run -p bidirectional-chat-client chat
# Select "Login with username/password"
# Enter credentials
```

Or use a saved token:
```bash
cargo run -p bidirectional-chat-client chat
# Select "Use existing token"
# Paste your JWT token
```

## Chat Commands

Once in the chat interface, you can use these commands:

- `/rooms` - List all available rooms
- `/join <room>` - Join or create a room
- `/leave` - Leave the current room
- `/help` - Show available commands
- `/quit` - Exit the chat client

To send a message, simply type and press Enter (without any slash command).

## Permissions

The chat system has three permission levels:

1. **User** (`user`):
   - Send messages
   - Join/leave rooms
   - List rooms

2. **Moderator** (`moderator`):
   - All user permissions
   - Kick users from the chat

3. **Admin** (`admin`):
   - All moderator permissions
   - Broadcast system-wide announcements

## WebSocket Protocol

The chat uses bidirectional JSON-RPC 2.0 over WebSockets:

### Client → Server Methods
- `send_message` - Send a chat message
- `join_room` - Join a chat room
- `leave_room` - Leave current room
- `list_rooms` - Get list of available rooms
- `kick_user` - Kick a user (moderator only)
- `broadcast_announcement` - Send system announcement (admin only)

### Server → Client Notifications
- `message_received` - New message in current room
- `user_joined` - User joined the room
- `user_left` - User left the room
- `system_announcement` - System-wide announcement
- `user_kicked` - User was kicked from chat
- `room_created` - New room was created
- `room_deleted` - Room was deleted

## Development

### Configuration

The server supports configuration through:
1. Configuration file (`config.toml`)
2. Environment variables (take precedence)
3. Command-line arguments (for future extension)

#### Configuration File

Copy `config.example.toml` to `config.toml` and modify as needed:

```bash
cp config.example.toml config.toml
```

Key configuration sections:
- **Server**: Host, port, and CORS settings
- **Auth**: JWT configuration and session management
- **Chat**: Message limits, room settings, persistence
- **Logging**: Log level and output format
- **Admin**: Initial admin users and permissions
- **Rate Limiting**: Request throttling (optional)

#### Environment Variables

Create a `.env` file in the server directory:
```env
# Core settings
JWT_SECRET=your-secret-key-here
HOST=0.0.0.0
PORT=3000

# Chat settings
CHAT_DATA_DIR=./chat_data
CHAT__CHAT__MAX_MESSAGE_LENGTH=1000
CHAT__CHAT__MAX_USERS_PER_ROOM=50

# Logging
RUST_LOG=info

# Admin users (example)
CHAT__ADMIN__USERS__0__USERNAME=admin
CHAT__ADMIN__USERS__0__PASSWORD=secure_password
```

See `config.example.toml` for a complete list of environment variables.

### Production Configuration

For production deployments:

1. **Security**:
   ```toml
   [auth]
   jwt_secret = "$(openssl rand -base64 32)"  # Generate secure secret
   jwt_ttl_seconds = 3600  # Shorter TTL for production
   
   [server.cors]
   allow_any_origin = false
   allowed_origins = ["https://yourchatapp.com"]
   ```

2. **Rate Limiting**:
   ```toml
   [rate_limit]
   enabled = true
   messages_per_minute = 10
   connections_per_ip = 5
   login_attempts_per_hour = 10
   ```

3. **Persistence**:
   ```toml
   [chat]
   data_dir = "/var/lib/chat-server/data"
   persist_messages = true
   persist_rooms = true
   persist_profiles = true
   ```

### Running Multiple Clients

You can run multiple client instances to simulate multiple users:

```bash
# Terminal 1
cargo run -p bidirectional-chat-client chat
# Login as alice

# Terminal 2  
cargo run -p bidirectional-chat-client chat
# Login as bob
```

### Testing Admin Features

Login as admin to test moderation features:
```bash
cargo run -p bidirectional-chat-client chat
# Username: admin
# Password: admin123
```

Then in another terminal as a regular user, you can be kicked by the admin.

## Implementation Details

### Authentication Flow
1. Client sends credentials to `/auth/login` endpoint
2. Server validates credentials and returns JWT
3. Client connects to WebSocket with JWT in Authorization header
4. Server validates JWT and establishes authenticated connection

### Message Flow
1. Client sends message via `send_message` RPC call
2. Server validates permissions and room membership
3. Server broadcasts `message_received` notification to all room members
4. Clients display the message in real-time

### Connection Lifecycle
1. **Connect**: Client establishes WebSocket connection
2. **Authenticate**: JWT validation during handshake
3. **Join Room**: Client must join a room to participate
4. **Interact**: Send messages, receive notifications
5. **Disconnect**: Automatic cleanup and notifications

## Troubleshooting

### Connection Issues
- Ensure server is running on `localhost:3000`
- Check firewall settings for WebSocket connections
- Verify JWT token hasn't expired (24-hour TTL by default)

### Authentication Errors
- Ensure you've registered the user first
- Check username/password are correct
- Verify JWT_SECRET matches between server restarts

### Message Not Sending
- Ensure you've joined a room first
- Check you have the required permissions
- Verify WebSocket connection is active