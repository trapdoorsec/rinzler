#!/bin/bash
# Version bumping script for Rinzler
# Usage: ./bump-version.sh [major|minor|patch|set VERSION]

set -e

CARGO_TOML="Cargo.toml"
RINZLER_CARGO_TOML="rinzler/Cargo.toml"
RINZLER_CORE_CARGO_TOML="rinzler-core/Cargo.toml"

# Extract current version from workspace Cargo.toml
CURRENT_VERSION=$(grep -m 1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo "Error: Could not find version in $CARGO_TOML"
    exit 1
fi

# Parse version components
# Strip timestamp if present (format: MAJOR.MINOR.PATCH-PRERELEASE-TIMESTAMP)
BASE_VERSION=$(echo "$CURRENT_VERSION" | sed 's/-[0-9]\{12\}$//')

# Extract MAJOR.MINOR.PATCH and optional PRERELEASE
if [[ $BASE_VERSION =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-[a-z]+)?$ ]]; then
    MAJOR="${BASH_REMATCH[1]}"
    MINOR="${BASH_REMATCH[2]}"
    PATCH="${BASH_REMATCH[3]}"
    PRERELEASE="${BASH_REMATCH[4]}"
else
    echo "Error: Version format not recognized: $CURRENT_VERSION"
    echo "Expected format: MAJOR.MINOR.PATCH[-PRERELEASE][-TIMESTAMP]"
    exit 1
fi

# Determine new version
case "$1" in
    major)
        NEW_MAJOR=$((MAJOR + 1))
        NEW_VERSION="${NEW_MAJOR}.0.0${PRERELEASE}"
        ;;
    minor)
        NEW_MINOR=$((MINOR + 1))
        NEW_VERSION="${MAJOR}.${NEW_MINOR}.0${PRERELEASE}"
        ;;
    patch)
        NEW_PATCH=$((PATCH + 1))
        NEW_VERSION="${MAJOR}.${MINOR}.${NEW_PATCH}${PRERELEASE}"
        ;;
    set)
        if [ -z "$2" ]; then
            echo "Error: Version not specified"
            echo "Usage: $0 set VERSION"
            exit 1
        fi
        NEW_VERSION="$2"
        # Validate format
        if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-z]+)?$ ]]; then
            echo "Error: Invalid version format: $NEW_VERSION"
            echo "Expected format: MAJOR.MINOR.PATCH[-PRERELEASE]"
            exit 1
        fi
        ;;
    *)
        echo "Usage: $0 [major|minor|patch|set VERSION]"
        echo ""
        echo "Examples:"
        echo "  $0 patch          # 0.1.2 -> 0.1.3"
        echo "  $0 minor          # 0.1.2 -> 0.2.0"
        echo "  $0 major          # 0.1.2 -> 1.0.0"
        echo "  $0 set 1.0.0-rc   # Set to specific version"
        exit 1
        ;;
esac

echo "Updating version: $CURRENT_VERSION -> $NEW_VERSION"

# Update workspace Cargo.toml
sed -i "0,/^version = \".*\"/{s/^version = \".*\"/version = \"$NEW_VERSION\"/}" "$CARGO_TOML"

# Update dependency versions in rinzler/Cargo.toml
if [ -f "$RINZLER_CARGO_TOML" ]; then
    echo "Updating dependency versions in $RINZLER_CARGO_TOML"
    sed -i "s/^\(rinzler-core = { version = \"\)[^\"]*\(\".*\)$/\1$NEW_VERSION\2/" "$RINZLER_CARGO_TOML"
    sed -i "s/^\(rinzler-scanner = { version = \"\)[^\"]*\(\".*\)$/\1$NEW_VERSION\2/" "$RINZLER_CARGO_TOML"
fi

# Update dependency versions in rinzler-core/Cargo.toml
if [ -f "$RINZLER_CORE_CARGO_TOML" ]; then
    echo "Updating dependency versions in $RINZLER_CORE_CARGO_TOML"
    sed -i "s/^\(rinzler-scanner = { version = \"\)[^\"]*\(\".*\)$/\1$NEW_VERSION\2/" "$RINZLER_CORE_CARGO_TOML"
fi

# Update Cargo.lock
echo "Updating Cargo.lock..."
cargo update --workspace --quiet 2>/dev/null || true

echo "Version bump complete: $NEW_VERSION"
