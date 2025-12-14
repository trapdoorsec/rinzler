# Scripts

## install-hooks.sh

Installs git hooks for the rinzler project.

### Usage

```bash
./scripts/install-hooks.sh
```

### What it does

Currently installs a pre-commit hook that automatically increments the patch/build version number (the third number in `MAJOR.MINOR.PATCH`) on each commit.

**Example:**
- Before commit: `0.1.1-alpha`
- After commit: `0.1.2-alpha`

The hook:
1. Reads the current version from workspace `Cargo.toml`
2. Increments the patch number
3. Preserves any pre-release suffix (e.g., `-alpha`, `-beta`)
4. Updates workspace `Cargo.toml` with the new version
5. Updates dependency versions in `rinzler/Cargo.toml` for:
   - `rinzler-core`
   - `rinzler-scanner`
6. Runs `cargo update --workspace` to update `Cargo.lock`
7. Stages all modified files automatically

### Disabling the hook

To temporarily disable version auto-increment:

```bash
# Remove the hook
rm .git/hooks/pre-commit

# Or skip it for a single commit
git commit --no-verify
```

### Manual version management

If you need to manually set a version (e.g., for a major/minor bump):

1. Edit the workspace `Cargo.toml` to set the new version
2. Edit `rinzler/Cargo.toml` to update the three dependency versions to match
3. Run `cargo update --workspace` to update Cargo.lock
4. Commit with the hook (it will continue incrementing from the new version)

**Example:** Bumping from `0.1.x` to `0.2.0`:
```bash
# Edit Cargo.toml: version = "0.2.0-alpha"
# Edit rinzler/Cargo.toml:
#   rinzler-core = { version = "0.2.0-alpha", ... }
#   rinzler-scanner = { version = "0.2.0-alpha", ... }
cargo update --workspace
git add Cargo.toml rinzler/Cargo.toml Cargo.lock
git commit -m "Bump version to 0.2.0-alpha"
# Next commit will auto-increment to 0.2.1-alpha
```
