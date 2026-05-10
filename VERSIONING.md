# Versioning and Changelog Policy

This workspace uses independent crate versions. Bump only the publishable crates
whose public API, generated output, runtime behavior, or documented behavior
changed.

## Versioning

- Follow SemVer for crates at `1.0.0` and later.
- While crates are `0.x`, use patch bumps for backward-compatible fixes,
  additive behavior, docs, and tooling improvements.
- While crates are `0.x`, use minor bumps for breaking public API changes or
  generated-code contract changes.
- Do not version-bump examples, fixtures, or crates marked `publish = false`
  unless their version is meaningful to a downstream release process.
- Keep `Cargo.lock` aligned with any package version changes.

## Changelog

- Every user-facing change must add an entry under `CHANGELOG.md` ->
  `[Unreleased]`.
- Group entries by `Added`, `Changed`, `Fixed`, `Documentation`, or
  `Maintenance`.
- Include the affected crate names when a change is scoped to specific crates.
- Mention version bumps in the same changelog entry set.

## Release Checklist

1. Move relevant `[Unreleased]` entries into a dated release section.
2. Confirm publishable crate versions match the release contents.
3. Run the relevant crate tests and any affected e2e suites.
4. Commit `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and release notes together.
5. Tag releases using crate-aware tags when publishing one crate, for example
   `ras-rest-macro-v0.1.1`.
