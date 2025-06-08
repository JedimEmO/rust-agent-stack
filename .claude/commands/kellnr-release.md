# Kellnr Release Command

This command guides you through the complete A-Z process of releasing the rust-agent-stack workspace to your local kellnr registry.

## Prerequisites Checklist

Before starting a release, ensure:
1. Kellnr registry is running at `http://localhost:8000/`
2. All tests are passing
3. No uncommitted changes exist

## Release Process

### Step 1: Verify Registry Health

First, check that kellnr is accessible:

```bash
curl -I http://localhost:8000/api/v1/crates/
```

Expected: HTTP 200 or 401 (if auth required)

### Step 2: Clean Build Verification

Ensure the workspace builds cleanly:

```bash
cargo clean
cargo build --all-targets
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

### Step 3: Version Management

Determine the new version number. Current version is in root `Cargo.toml`.

Version bump types:
- **Patch** (0.1.0 → 0.1.1): Bug fixes, minor changes
- **Minor** (0.1.0 → 0.2.0): New features, backwards compatible
- **Major** (0.1.0 → 1.0.0): Breaking changes

Update ALL crate versions simultaneously:
1. Update root `Cargo.toml` workspace version
2. Update each crate's `Cargo.toml` version
3. Update internal dependency versions to match

Files to update:
- `/home/mmy/repos/ai/rust-agent-stack/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/libs/rust-jsonrpc-types/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/libs/rust-jsonrpc-core/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/libs/rust-jsonrpc-macro/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/libs/openrpc-types/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-core/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-local/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-oauth2/Cargo.toml`
- `/home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-session/Cargo.toml`

### Step 4: Update Internal Dependencies

After updating versions, ensure all internal dependencies reference the new version.

For example, if bumping to `0.2.0`, update:
```toml
rust-identity-core = { path = "../rust-identity-core", version = "0.2.0" }
```

### Step 5: Verify Dependency Resolution

Check that all dependencies resolve correctly:

```bash
cargo check --workspace
cargo tree --workspace
```

### Step 6: Final Test Run

Run all tests with the new versions:

```bash
cargo test --workspace --all-features
```

### Step 7: Commit Version Changes

Create a version bump commit:

```bash
git add -A
git commit -m "chore: bump version to X.Y.Z for kellnr release"
```

### Step 8: Tag the Release

Create a git tag for the release:

```bash
git tag -a vX.Y.Z -m "Release version X.Y.Z"
```

### Step 9: Publish to Kellnr

Publish crates in dependency order. The order is critical due to inter-dependencies:

#### Layer 0 (No internal dependencies):
```bash
cd /home/mmy/repos/ai/rust-agent-stack/crates/libs/rust-jsonrpc-types
cargo publish --registry kellnr

cd /home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-core
cargo publish --registry kellnr
```

#### Layer 1 (Depends on Layer 0):
```bash
cd /home/mmy/repos/ai/rust-agent-stack/crates/libs/rust-jsonrpc-core
cargo publish --registry kellnr

cd /home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-local
cargo publish --registry kellnr

cd /home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-oauth2
cargo publish --registry kellnr
```

#### Layer 2 (Depends on Layer 1):
```bash
cd /home/mmy/repos/ai/rust-agent-stack/crates/libs/rust-jsonrpc-macro
cargo publish --registry kellnr

cd /home/mmy/repos/ai/rust-agent-stack/crates/identity/rust-identity-session
cargo publish --registry kellnr
```

#### Layer 3 (Depends on Layer 2):
```bash
cd /home/mmy/repos/ai/rust-agent-stack/crates/libs/openrpc-types
cargo publish --registry kellnr
```

### Step 10: Verify Published Crates

Check that all crates are available in kellnr:

```bash
cargo search --registry kellnr rust-jsonrpc
cargo search --registry kellnr rust-identity
cargo search --registry kellnr openrpc-types
```

### Step 11: Post-Release Tasks

1. Push the commit and tag:
```bash
git push origin main
git push origin vX.Y.Z
```

2. Update CHANGELOG.md with release notes

3. Consider creating a release announcement

## Troubleshooting

### Common Issues

#### Authentication Errors
- Ensure token is in `~/.cargo/credentials`
- Format: `[registries.kellnr]\ntoken = "your-token-here"`

#### Dependency Resolution Failures
- Check all internal versions match
- Ensure publishing order is correct
- Wait a few seconds between publishes for index updates

#### Publishing Failures
- Check kellnr logs for errors
- Verify crate doesn't already exist at that version
- Ensure all required fields in Cargo.toml are present

### Rollback Procedure

If something goes wrong:

1. Yank the problematic version:
```bash
cargo yank --vers X.Y.Z --registry kellnr <crate-name>
```

2. Fix the issue and publish a new patch version

3. Document the issue in CHANGELOG.md

## Registry Configuration Reference

Current kellnr configuration in `.cargo/config.toml`:
- Registry URL: `http://localhost:8000/api/v1/crates/`
- Index URL: `http://localhost:8000/crate-index/`
- Default registry: `kellnr`

## Version Dependencies Matrix

When updating versions, ensure all these dependencies are synchronized:

| Crate | Depends On |
|-------|------------|
| rust-jsonrpc-types | None |
| rust-identity-core | None |
| rust-jsonrpc-core | rust-jsonrpc-types |
| rust-identity-local | rust-identity-core |
| rust-identity-oauth2 | rust-identity-core |
| rust-jsonrpc-macro | rust-jsonrpc-core, rust-jsonrpc-types |
| rust-identity-session | rust-identity-core, rust-jsonrpc-core |
| openrpc-types | None |

## Notes

- Always publish in dependency order
- All crates should have the same version for simplicity
- Examples are marked with `publish = false` and won't be published
- The entire release process typically takes 10-15 minutes
- Consider automating this process with CI/CD in the future