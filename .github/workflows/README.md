# GitHub Actions Security Workflows

This directory contains security-focused GitHub Actions workflows for the Rust Agent Stack project.

## Workflows Overview

### 1. Security Audit (`security-audit.yml`)
- **Purpose**: Scans dependencies for known security vulnerabilities
- **Tools**: cargo-audit, cargo-outdated
- **Schedule**: Runs on push, PR, and weekly (Mondays at 8:00 UTC)
- **Features**:
  - Checks for vulnerable dependencies using RustSec advisory database
  - Identifies outdated dependencies that may have security patches
  - Generates JSON reports for further analysis

### 2. Cargo Deny (`cargo-deny.yml`)
- **Purpose**: Supply chain security and license compliance
- **Tools**: cargo-deny
- **Schedule**: Runs on push, PR, and weekly (Mondays at 9:00 UTC)
- **Checks**:
  - **Advisories**: Security vulnerabilities
  - **Bans**: Banned or duplicate dependencies
  - **Licenses**: License compliance
  - **Sources**: Trusted sources verification
- **Configuration**: Uses `deny.toml` in repository root

### 3. Code Security Analysis (`code-security.yml`)
- **Purpose**: Static code analysis for security issues
- **Tools**: Clippy, Semgrep, custom security checks
- **Features**:
  - Enhanced Clippy lints for security (panic, unwrap, indexing)
  - Semgrep rules for Rust, JWT, OWASP Top 10
  - Checks for unsafe code blocks
  - Scans for hardcoded secrets
  - Identifies security-related TODOs

### 4. Security SARIF Upload (`security-sarif.yml`)
- **Purpose**: Integration with GitHub Security tab
- **Tools**: clippy-sarif, cargo-audit-sarif, Trivy
- **Schedule**: Runs on push, PR, and weekly (Mondays at 10:00 UTC)
- **Features**:
  - Converts security findings to SARIF format
  - Uploads results to GitHub Security tab
  - Includes multiple security scanners:
    - Clippy security lints
    - Cargo audit vulnerabilities
    - Trivy filesystem scanning

### 5. Auth & WASM Security (`auth-wasm-security.yml`)
- **Purpose**: Specialized security checks for authentication and WASM code
- **Triggers**: Only runs when auth or WASM files are modified
- **Checks**:
  - Hardcoded secrets in authentication code
  - Weak cryptographic functions
  - JWT security (algorithm validation, expiration)
  - OAuth2 security (PKCE, state parameter)
  - WASM bundle size and optimization
  - Unsafe WASM patterns (innerHTML, eval)
  - Security policy file existence

## Configuration Files

### `deny.toml`
Configuration for cargo-deny with:
- Allowed/denied licenses (MIT, Apache-2.0 allowed; GPL denied)
- Advisory database settings
- Source repository allowlists
- Ban rules for problematic dependencies

## Security Best Practices

1. **Review all security warnings** before merging PRs
2. **Update dependencies regularly** to get security patches
3. **Check SARIF results** in the GitHub Security tab
4. **Address high/critical vulnerabilities** immediately
5. **Monitor authentication code** changes carefully
6. **Validate WASM security** before deploying to production

## Customization

To customize these workflows:
1. Edit the schedule in the `on.schedule.cron` field
2. Adjust security lint levels in Clippy commands
3. Add/remove licenses in `deny.toml`
4. Configure Semgrep rules in `code-security.yml`
5. Add custom security checks as needed

## Troubleshooting

- **Workflow failures**: Check the Actions tab for detailed logs
- **False positives**: Add exceptions to `deny.toml` or inline comments
- **Performance issues**: Adjust cache settings or run workflows less frequently
- **SARIF upload errors**: Ensure proper permissions in workflow files