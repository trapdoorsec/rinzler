# Release Workflow

This document describes the release process for Rinzler.

## Quick Release (Recommended)

For most releases, use the automated workflow:

```bash
make publish-full
```

This interactive command will:
1. Ask you what type of version bump (major/minor/patch/custom)
2. Update all version numbers
3. Run the full CI pipeline (format, lint, test)
4. Run a dry-run publish check
5. Ask for confirmation
6. Commit the version changes
7. Create and push a git tag
8. Publish to crates.io
9. Push commits to remote

**Example session:**
```
$ make publish-full
Current version: 0.1.12-alpha
What type of release?
  1) Patch (0.1.2 -> 0.1.3)
  2) Minor (0.1.2 -> 0.2.0)
  3) Major (0.1.2 -> 1.0.0)
  4) Custom version

Select [1-4]: 1

Version updated to 0.1.13-alpha

Running full test suite...
[tests run...]

Running dry-run publish...
[dry-run checks...]

Ready to publish version 0.1.13-alpha
This will:
  1. Commit version changes
  2. Create and push git tag v0.1.13-alpha
  3. Publish to crates.io
  4. Push commits to remote

Proceed with release? [y/N]: y
```

## Manual Release Process

If you need more control, you can run each step manually:

### 1. Version Bumping

```bash
# Bump version
make bump-patch   # 0.1.2 -> 0.1.3
make bump-minor   # 0.1.2 -> 0.2.0
make bump-major   # 0.1.2 -> 1.0.0

# Or set a specific version
make version-set VERSION=1.0.0-beta
```

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
git add Cargo.toml Cargo.lock rinzler/Cargo.toml rinzler-core/Cargo.toml
git commit -m "Release vX.Y.Z" --no-verify
```

Note: Use `--no-verify` to skip the pre-commit hook that adds timestamps.

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

## Development vs Release Versions

### Development Builds
- Git pre-commit hook automatically appends timestamp
- Format: `0.1.12-alpha-251215143022`
- Happens on every commit during development
- Provides unique, sortable version for each commit

### Release Builds
- Manual version bumps via Makefile
- Format: `0.1.13-alpha` (no timestamp)
- Clean semantic versions for published releases
- Commit with `--no-verify` to skip timestamp hook

## Pre-Release Checklist

Before running `make publish-full`:

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
