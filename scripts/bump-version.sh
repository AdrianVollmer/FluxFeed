#!/usr/bin/env bash
set -euo pipefail

# FluxFeed Version Bump Script
#
# Usage: ./scripts/bump-version.sh <new-version>
# Example: ./scripts/bump-version.sh 0.2.0
#
# This script:
# 1. Updates version in Cargo.toml
# 2. Updates version in package.json
# 3. Updates Cargo.lock
# 4. Creates a git commit
# 5. Creates a git tag (v<version>)

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

NEW_VERSION="$1"

# Validate version format (semantic versioning: X.Y.Z)
if ! echo "$NEW_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: Version must be in format X.Y.Z (e.g., 0.2.0)"
    exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -n1 | sed 's/version = "\(.*\)"/\1/')

echo "Bumping version from $CURRENT_VERSION to $NEW_VERSION"

# Update Cargo.toml
echo "Updating Cargo.toml..."
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update package.json
echo "Updating package.json..."
sed -i.bak "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" package.json
rm package.json.bak

# Update Cargo.lock
echo "Updating Cargo.lock..."
cargo check --quiet

# Check if there are changes
if ! git diff --quiet Cargo.toml package.json Cargo.lock; then
    echo "Creating git commit and tag..."
    git add Cargo.toml package.json Cargo.lock
    git -c user.name="Claude Code" -c user.email="noreply@anthropic.com" commit -m "$(cat <<EOF
Bump version to $NEW_VERSION

Updated version number across:
- Cargo.toml
- package.json
- Cargo.lock

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
EOF
)"

    # Create git tag
    git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

    echo ""
    echo "✓ Version bumped to $NEW_VERSION"
    echo "✓ Commit created"
    echo "✓ Tag v$NEW_VERSION created"
    echo ""
    echo "To push the changes and trigger Docker build:"
    echo "  git push origin main"
    echo "  git push origin v$NEW_VERSION"
else
    echo "No changes detected. Version might already be $NEW_VERSION"
    exit 1
fi
