#!/bin/bash
# Install git hooks for rinzler project

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks..."

# Create pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Auto-update version with timestamp on each commit

CARGO_TOML="Cargo.toml"
RINZLER_CARGO_TOML="rinzler/Cargo.toml"
RINZLER_CORE_CARGO_TOML="rinzler-core/Cargo.toml"

# Extract current version from workspace Cargo.toml
CURRENT_VERSION=$(grep -m 1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo "Error: Could not find version in $CARGO_TOML"
    exit 1
fi

# Generate timestamp in YYMMDDHHMMSS format
TIMESTAMP=$(date +%y%m%d%H%M%S)

# Split version into components
# Format: MAJOR.MINOR.PATCH-PRERELEASE-TIMESTAMP or MAJOR.MINOR.PATCH-PRERELEASE
if [[ $CURRENT_VERSION =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-[a-z]+)(-[0-9]+)?$ ]]; then
    MAJOR="${BASH_REMATCH[1]}"
    MINOR="${BASH_REMATCH[2]}"
    PATCH="${BASH_REMATCH[3]}"
    PRERELEASE="${BASH_REMATCH[4]}"  # e.g., "-alpha"

    # Create new version with timestamp
    NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}${PRERELEASE}-${TIMESTAMP}"

    echo "Updating version: $CURRENT_VERSION → $NEW_VERSION"

    # Update version in workspace Cargo.toml (only the first occurrence in [workspace.package])
    sed -i "0,/^version = \".*\"/{s/^version = \".*\"/version = \"$NEW_VERSION\"/}" "$CARGO_TOML"

    # Update dependency versions in rinzler/Cargo.toml
    if [ -f "$RINZLER_CARGO_TOML" ]; then
        echo "Updating dependency versions in $RINZLER_CARGO_TOML"
        sed -i "s/^\(rinzler-core = { version = \"\)[^\"]*\(\".*\)$/\1$NEW_VERSION\2/" "$RINZLER_CARGO_TOML"
        sed -i "s/^\(rinzler-scanner = { version = \"\)[^\"]*\(\".*\)$/\1$NEW_VERSION\2/" "$RINZLER_CARGO_TOML"
        git add "$RINZLER_CARGO_TOML"
    fi

    # Update dependency versions in rinzler-core/Cargo.toml
    if [ -f "$RINZLER_CORE_CARGO_TOML" ]; then
        echo "Updating dependency versions in $RINZLER_CORE_CARGO_TOML"
        sed -i "s/^\(rinzler-scanner = { version = \"\)[^\"]*\(\".*\)$/\1$NEW_VERSION\2/" "$RINZLER_CORE_CARGO_TOML"
        git add "$RINZLER_CORE_CARGO_TOML"
    fi

    # Update Cargo.lock
    cargo update --workspace --quiet 2>/dev/null || true

    # Stage the updated files
    git add "$CARGO_TOML" Cargo.lock

    echo "Version updated successfully"
else
    echo "Warning: Version format not recognized: $CURRENT_VERSION"
    echo "Expected format: MAJOR.MINOR.PATCH-PRERELEASE[-TIMESTAMP]"
    exit 1
fi

exit 0
EOF

chmod +x "$HOOKS_DIR/pre-commit"

echo "✓ pre-commit hook installed"
echo ""
echo "The hook will automatically update the version with a timestamp on each commit."
echo "Current version: $(grep -m 1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')"
