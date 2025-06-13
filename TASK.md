# Bidirectional Chat Application with Ratatui Terminal UI

## Summary

Build a fully-featured chat application using the existing bidirectional RPC infrastructure, local authentication system, and create a beautiful terminal UI client using ratatui with animated ASCII art cat avatars. The project will demonstrate the full capabilities of the rust-agent-stack's bidirectional WebSocket communication, JWT-based authentication, and permission system while providing an engaging user experience through animated cat avatars in the terminal.

Key deliverables:
- Standalone chat server using bidirectional RPC with room support
- Terminal UI client with ratatui featuring animated ASCII cat avatars
- User registration and login via local auth provider
- Real-time messaging with typing indicators
- Admin commands for user management
- Persistent chat history and user profiles

## Sprint 1: Foundation and Server Implementation

- [x] Extract and refactor the ChatService from test code into a standalone server example
- [x] Create `examples/bidirectional-chat-server/` with proper Cargo.toml configuration
- [x] Implement persistent state management for chat rooms and message history
- [x] Add user profile support with avatar selection (cat breed/color preferences)
- [x] Create shared types module for client-server communication
- [x] Implement comprehensive logging and error handling
- [x] Add server configuration via environment variables or config file
- [x] Create integration tests for all server endpoints

## Sprint 2: Basic Terminal Client Implementation

- [ ] Create `examples/bidirectional-chat-client/` with ratatui dependencies
- [ ] Implement basic terminal UI layout with message area, input area, and user list
- [ ] Create CLI command structure (register, login, chat, logout)
- [ ] Implement user registration flow with local auth provider
- [ ] Implement login flow with JWT token storage and management
- [ ] Add basic message sending and receiving functionality
- [ ] Implement connection status indicators and reconnection handling
- [ ] Create help screens and keyboard shortcut overlays

## Sprint 3: ASCII Art Cat System

- [ ] Design ASCII art cat face components (eyes, ears, mouth, whiskers)
- [ ] Create cat animation system with different expressions (happy, sad, thinking, sleeping)
- [ ] Implement cat breed variations (tabby, siamese, persian, etc.)
- [ ] Create typing animation where cat's mouth moves while user types
- [ ] Add idle animations (blinking, ear twitching, tail swaying)
- [ ] Implement avatar customization screen in the client
- [ ] Create cat reaction system for different message types (emojis, mentions)
- [ ] Add ASCII art library with at least 10 different cat variations

## Sprint 4: Advanced Chat Features

- [ ] Implement chat rooms/channels with join/leave functionality
- [ ] Add private messaging between users
- [ ] Create typing indicators showing which users are currently typing
- [ ] Implement message history with scrollback and search
- [ ] Add emoji support with cat-themed custom emoji reactions
- [ ] Create user presence system (online, away, busy status)
- [ ] Implement message formatting (bold, italic, code blocks)
- [ ] Add notification system for mentions and private messages

## Sprint 5: Admin Features and Polish

- [ ] Implement admin command interface (kick, ban, mute users)
- [ ] Add moderation tools (message deletion, word filters)
- [ ] Create user profile viewing and management screens
- [ ] Implement chat statistics dashboard (messages per day, active users)
- [ ] Add theme customization (color schemes for the UI)
- [ ] Create comprehensive help documentation within the app
- [ ] Implement export functionality for chat logs
- [ ] Add sound notifications (optional terminal bell)

## Sprint 6: Performance and Production Readiness

- [ ] Optimize message rendering for large chat histories
- [ ] Implement efficient user list updates with delta synchronization
- [ ] Add connection pooling and rate limiting on the server
- [ ] Create deployment scripts and Docker containers
- [ ] Write comprehensive user documentation and README
- [ ] Implement automated tests for the terminal UI
- [ ] Add performance monitoring and metrics collection
- [ ] Create demo video showcasing all features with cat animations