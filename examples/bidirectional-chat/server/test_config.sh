#!/bin/bash

echo "Testing Bidirectional Chat Server Configuration"
echo "=============================================="

# Test 1: Default configuration from file
echo -e "\n1. Testing with config.toml:"
echo "Expected: port 3000, debug logging, test-room"
timeout 3 cargo run --release 2>&1 | grep -E "(Configuration loaded|listening|WebSocket|room|testadmin)" &

sleep 5
echo -e "\n\n2. Testing with environment overrides:"
echo "Expected: port 4000, info logging"
CHAT__SERVER__PORT=4000 CHAT__LOGGING__LEVEL=info timeout 3 cargo run --release 2>&1 | grep -E "(Configuration loaded|listening|4000)" &

sleep 5
echo -e "\n\n3. Testing with JWT_SECRET override:"
echo "Expected: custom JWT secret warning should not appear"
JWT_SECRET=production-secret timeout 3 cargo run --release 2>&1 | grep -E "(JWT|secret)" &

sleep 5
echo -e "\n\nDone!"