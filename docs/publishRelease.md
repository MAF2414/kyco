# Publishing Releases

This document describes how to build and publish IDE extension releases for KYCo.

## Prerequisites

- Node.js and npm installed
- Java 17+ installed (for JetBrains plugin)
- GitHub CLI (`gh`) installed and authenticated
- `@vscode/vsce` installed globally: `npm install -g @vscode/vsce`

## Version Bumping

Before building, update the version in both extensions:

### VSCode Extension
Edit `vscode-extension/package.json`:
```json
"version": "X.Y.Z"
```

### JetBrains Plugin
Edit `jetbrains-plugin/build.gradle.kts`:
```kotlin
version = "X.Y.Z"
// Also update in pluginConfiguration block:
pluginConfiguration {
    version = "X.Y.Z"
}
```

## Building

### VSCode Extension

```bash
cd vscode-extension

# Compile TypeScript
npm run compile

# Build VSIX package
vsce package

# Output: kyco-X.Y.Z.vsix
```

### JetBrains Plugin

```bash
cd jetbrains-plugin

# Build plugin ZIP
./gradlew buildPlugin

# Output: build/distributions/kyco-jetbrains-plugin-X.Y.Z.zip
```

## Creating GitHub Release

### 1. Prepare artifacts with generic names

The download URLs use generic names (without version) so they work with `/latest/download/`:

```bash
cd vscode-extension
cp kyco-X.Y.Z.vsix kyco-vscode.vsix

cd ../jetbrains-plugin/build/distributions
cp kyco-jetbrains-plugin-X.Y.Z.zip kyco-jetbrains.zip
```

### 2. Create the release

```bash
gh release create vX.Y.Z \
  --title "vX.Y.Z - IDE Extensions" \
  --notes "$(cat <<'EOF'
## IDE Extensions Release

### Changes
- List your changes here

### Installation
- **VSCode**: `code --install-extension kyco-X.Y.Z.vsix`
- **JetBrains**: Settings -> Plugins -> Install Plugin from Disk -> Select the .zip file
EOF
)" \
  vscode-extension/kyco-X.Y.Z.vsix \
  vscode-extension/kyco-vscode.vsix \
  jetbrains-plugin/build/distributions/kyco-jetbrains-plugin-X.Y.Z.zip \
  jetbrains-plugin/build/distributions/kyco-jetbrains.zip
```

Note: We upload both versioned and generic-named files:
- Versioned files (`kyco-X.Y.Z.vsix`) for users who want a specific version
- Generic files (`kyco-vscode.vsix`) for the `/latest/download/` URL pattern

## Download URLs

After release, the extensions are available at:

- **VSCode (latest)**: `https://github.com/MAF2414/kyco/releases/latest/download/kyco-vscode.vsix`
- **JetBrains (latest)**: `https://github.com/MAF2414/kyco/releases/latest/download/kyco-jetbrains.zip`

## Installation Commands

### VSCode
```bash
# Download and install latest
curl -L https://github.com/MAF2414/kyco/releases/latest/download/kyco-vscode.vsix -o kyco-vscode.vsix
code --install-extension kyco-vscode.vsix
```

### JetBrains
1. Download from GitHub releases
2. In IDE: Settings -> Plugins -> Gear icon -> Install Plugin from Disk
3. Select the downloaded `.zip` file
4. Restart IDE

## Optional: Marketplace Publishing

### VSCode Marketplace (not currently used)

```bash
# Create PAT at https://dev.azure.com with Marketplace (Manage) scope
vsce login kyco
vsce publish
```

### JetBrains Marketplace (not currently used)

```bash
# Create token at https://plugins.jetbrains.com/author/me/tokens
./gradlew publishPlugin -Dorg.jetbrains.intellij.platform.publishing.token=YOUR_TOKEN
```
