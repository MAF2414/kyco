#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════
# Kyco Version Bump Script
# ═══════════════════════════════════════════════════════════════════════════
# Updates version in all project files:
#   - Cargo.toml
#   - vscode-extension/package.json
#   - jetbrains-plugin/build.gradle.kts (2 locations)
#
# Usage: ./scripts/bump-version.sh 0.4.0
# ═══════════════════════════════════════════════════════════════════════════

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

if [ -z "$1" ]; then
    echo -e "${RED}Error: Version argument required${NC}"
    echo "Usage: $0 <version>"
    echo "Example: $0 0.4.0"
    exit 1
fi

VERSION="$1"

# Validate version format (semver without v prefix)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo -e "${RED}Error: Invalid version format${NC}"
    echo "Expected format: X.Y.Z or X.Y.Z-suffix (e.g., 0.4.0, 1.0.0-beta)"
    exit 1
fi

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${YELLOW}Bumping version to ${VERSION}...${NC}"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Update Cargo.toml
# ─────────────────────────────────────────────────────────────────────────────
CARGO_FILE="$PROJECT_ROOT/Cargo.toml"
if [ -f "$CARGO_FILE" ]; then
    # Use sed to update version on line 3 (version = "x.y.z")
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_FILE"
    else
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_FILE"
    fi
    echo -e "${GREEN}✓${NC} Updated Cargo.toml"
else
    echo -e "${RED}✗${NC} Cargo.toml not found"
    exit 1
fi

# ─────────────────────────────────────────────────────────────────────────────
# Update vscode-extension/package.json
# ─────────────────────────────────────────────────────────────────────────────
VSCODE_FILE="$PROJECT_ROOT/vscode-extension/package.json"
if [ -f "$VSCODE_FILE" ]; then
    # Use sed to update "version": "x.y.z"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$VSCODE_FILE"
    else
        sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$VSCODE_FILE"
    fi
    echo -e "${GREEN}✓${NC} Updated vscode-extension/package.json"
else
    echo -e "${YELLOW}!${NC} vscode-extension/package.json not found (skipping)"
fi

# ─────────────────────────────────────────────────────────────────────────────
# Update jetbrains-plugin/build.gradle.kts
# ─────────────────────────────────────────────────────────────────────────────
JETBRAINS_FILE="$PROJECT_ROOT/jetbrains-plugin/build.gradle.kts"
if [ -f "$JETBRAINS_FILE" ]; then
    # Update version = "x.y.z" (appears twice: line 8 and line 32)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$JETBRAINS_FILE"
        # Also update the version inside pluginConfiguration block
        sed -i '' "s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$VERSION\"/g" "$JETBRAINS_FILE"
    else
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$JETBRAINS_FILE"
        sed -i "s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$VERSION\"/g" "$JETBRAINS_FILE"
    fi
    echo -e "${GREEN}✓${NC} Updated jetbrains-plugin/build.gradle.kts"
else
    echo -e "${YELLOW}!${NC} jetbrains-plugin/build.gradle.kts not found (skipping)"
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Version bumped to ${VERSION}${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Commit:         git add -A && git commit -m \"chore: bump version to $VERSION\""
echo "  3. Tag:            git tag v$VERSION"
echo "  4. Push:           git push && git push --tags"
echo ""
echo "GitHub Actions will automatically build and create a release."
