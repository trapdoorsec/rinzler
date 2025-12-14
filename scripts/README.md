# Scripts

## bump-version.sh

Bump or set the project version across all workspace crates.

### Usage

```bash
# Bump patch version (0.1.2 -> 0.1.3)
./scripts/bump-version.sh patch

# Bump minor version (0.1.2 -> 0.2.0)
./scripts/bump-version.sh minor

# Bump major version (0.1.2 -> 1.0.0)
./scripts/bump-version.sh major

# Set specific version
./scripts/bump-version.sh set 1.0.0-beta
```

### What it does

1. Extracts current version from workspace `Cargo.toml`
2. Strips any timestamp suffix if present
3. Calculates new version based on bump type
4. Updates workspace `Cargo.toml`
5. Updates all dependency versions in:
   - `rinzler/Cargo.toml` (rinzler-core, rinzler-scanner)
   - `rinzler-core/Cargo.toml` (rinzler-scanner)
6. Runs `cargo update --workspace` to update `Cargo.lock`

**Note:** This is typically called via Makefile targets rather than directly.

---

## install-hooks.sh

Installs git hooks for the rinzler project.

### Usage

```bash
./scripts/install-hooks.sh
```

### What it does

Installs a pre-commit hook that automatically updates the version with a timestamp on each commit.

**Example:**
- Before commit: `0.1.12-alpha-250115120000`
- After commit: `0.1.12-alpha-250115143022` (January 15, 2025 at 14:30:22)

The version format is: `MAJOR.MINOR.PATCH-PRERELEASE-YYMMDDHHMMSS`

The hook:
1. Reads the current version from workspace `Cargo.toml`
2. Generates a timestamp in YYMMDDHHMMSS format
3. Creates new version as `MAJOR.MINOR.PATCH-PRERELEASE-TIMESTAMP`
4. Updates workspace `Cargo.toml` with the new version
5. Updates dependency versions in `rinzler/Cargo.toml` for:
   - `rinzler-core`
   - `rinzler-scanner`
6. Updates dependency versions in `rinzler-core/Cargo.toml` for:
   - `rinzler-scanner`
7. Runs `cargo update --workspace` to update `Cargo.lock`
8. Stages all modified files automatically

### Disabling the hook

To temporarily disable version auto-increment:

```bash
# Remove the hook
rm .git/hooks/pre-commit

# Or skip it for a single commit
git commit --no-verify
```

### Manual version management

If you need to manually set a version (e.g., for a major/minor/patch bump):

1. Edit the workspace `Cargo.toml` to set the new version (without timestamp)
2. Edit `rinzler/Cargo.toml` to update the dependency versions to match
3. Edit `rinzler-core/Cargo.toml` to update the dependency version to match
4. Run `cargo update --workspace` to update Cargo.lock
5. Commit with the hook (it will append the timestamp automatically)

**Example:** Bumping from `0.1.x` to `0.2.0`:
```bash
# Edit Cargo.toml: version = "0.2.0-alpha"
# Edit rinzler/Cargo.toml:
#   rinzler-core = { version = "0.2.0-alpha", ... }
#   rinzler-scanner = { version = "0.2.0-alpha", ... }
# Edit rinzler-core/Cargo.toml:
#   rinzler-scanner = { version = "0.2.0-alpha", ... }
cargo update --workspace
git add Cargo.toml rinzler/Cargo.toml rinzler-core/Cargo.toml Cargo.lock
git commit -m "Bump version to 0.2.0-alpha"
# Hook will append timestamp: 0.2.0-alpha-250115143022
```

**Note:** The base version (without timestamp) should follow the format `MAJOR.MINOR.PATCH-PRERELEASE` where PRERELEASE is lowercase letters only (e.g., `-alpha`, `-beta`).
