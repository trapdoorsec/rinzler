# Release Workflow

This document describes the release process for Rinzler.

## Release Process

### 1. Update Version Numbers

Manually update the version in the workspace Cargo.toml:

```bash
# Edit Cargo.toml and update the version field
vim Cargo.toml

# Update the version in rinzler-core dependency reference
vim rinzler-core/Cargo.toml
```

Ensure all version numbers match across:
- `Cargo.toml` (workspace.package.version)
- `rinzler-core/Cargo.toml` (rinzler-scanner dependency version)

### 2. Run Tests

```bash
make ci
```

This runs:
- Format check (`cargo fmt -- --check`)
- Clippy lints (`cargo clippy`)
- All tests (`cargo test`)

### 3. Dry-Run Publish

```bash
make publish-dry
```

Verifies that all crates can be published without actually publishing.

### 4. Commit Version Changes

```bash
git add Cargo.toml Cargo.lock rinzler/Cargo.toml rinzler-core/Cargo.toml rinzler-scanner/Cargo.toml
git commit -m "Release vX.Y.Z"
```

### 5. Create and Push Tag

```bash
make tag-release
```

Or manually:
```bash
VERSION=$(grep -m 1 '^version' Cargo.toml | awk -F'"' '{print $2}')
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"
```

### 6. Publish to crates.io

```bash
make publish
```

This will publish in order:
1. rinzler-scanner (no dependencies)
2. Wait 30s for indexing
3. rinzler-core (depends on rinzler-scanner)
4. Wait 30s for indexing
5. rinzler (depends on both)

### 7. Push Commits

```bash
git push
```

## Pre-Release Checklist

Before publishing:

- [ ] Update CHANGELOG.md with release notes
- [ ] Ensure all tests pass locally (`make test`)
- [ ] Ensure code is formatted (`make fmt`)
- [ ] Ensure no clippy warnings (`make clippy`)
- [ ] Update README.md if needed
- [ ] Review dependency versions
- [ ] Ensure you're on the main branch
- [ ] Ensure working directory is clean (`git status`)

## Post-Release

After a successful release:

1. Verify the release on crates.io:
   - https://crates.io/crates/rinzler-scanner
   - https://crates.io/crates/rinzler-core
   - https://crates.io/crates/rinzler

2. Create a GitHub release from the tag with release notes

3. Announce the release (if applicable)

## Troubleshooting

### Publish failed midway

If publishing fails after some crates are published:

```bash
# Continue from where it failed
cd rinzler-core && cargo publish
sleep 30
cd ../rinzler && cargo publish
```

### Need to unpublish/yank

```bash
cargo yank --vers X.Y.Z rinzler
cargo yank --vers X.Y.Z rinzler-core
cargo yank --vers X.Y.Z rinzler-scanner
```

### Version mismatch errors

If you get version mismatch errors, ensure all Cargo.toml files have matching versions:

```bash
grep -r "version.*=.*\"" --include="Cargo.toml" .
```

## Useful Commands

```bash
make version              # Show current version
make publish-dry          # Test publish without actually publishing
make tag-release          # Create and push git tag
cargo search rinzler      # Check published versions on crates.io
```
