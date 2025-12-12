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
1. Reads the current version from `Cargo.toml`
2. Increments the patch number
3. Preserves any pre-release suffix (e.g., `-alpha`, `-beta`)
4. Updates `Cargo.toml` with the new version
5. Runs `cargo update --workspace` to update `Cargo.lock`
6. Stages both files automatically

### Disabling the hook

To temporarily disable version auto-increment:

```bash
# Remove the hook
rm .git/hooks/pre-commit

# Or skip it for a single commit
git commit --no-verify
```

### Manual version management

If you need to manually set a version (e.g., for a major/minor bump), edit `Cargo.toml` directly and the hook will continue incrementing from that new version.
