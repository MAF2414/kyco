# Publishing a New Kyco Release

This document describes how to create and publish a new Kyco release using the automated CI/CD pipeline.

## Quick Start

```bash
# 1. Bump version
./scripts/bump-version.sh 0.4.0

# 2. Commit & Tag
git add -A
git commit -m "chore: bump version to 0.4.0"
git tag v0.4.0

# 3. Push (triggers automated build)
git push && git push --tags
```

GitHub Actions will automatically build everything and create a release!

---

## Prerequisites

- Write access to the repository
- Git configured with push access

## Detailed Release Process

### 1. Bump Version

Use the version bump script to update the version in all project files:

```bash
./scripts/bump-version.sh 0.4.0
```

This updates the version in:
- `Cargo.toml` (Rust binary)
- `vscode-extension/package.json` (VS Code extension)
- `jetbrains-plugin/build.gradle.kts` (JetBrains plugin)

### 2. Review Changes

Verify the version changes:

```bash
git diff
```

### 3. Commit and Tag

```bash
git add -A
git commit -m "chore: bump version to 0.4.0"
git tag v0.4.0
```

### 4. Push to GitHub

```bash
git push && git push --tags
```

### 5. Automated Build (GitHub Actions)

The CI/CD pipeline automatically:

1. **Build VS Code Extension** (Ubuntu, Node.js 20)
   - Compile TypeScript
   - Package VSIX

2. **Build JetBrains Plugin** (Ubuntu, Java 17)
   - Run Gradle buildPlugin
   - Package ZIP

3. **Build Rust Binaries** (Matrix build)
   - macOS x64 (`x86_64-apple-darwin`)
   - macOS ARM (`aarch64-apple-darwin`)
   - Linux x64 (`x86_64-unknown-linux-gnu`)
   - Windows x64 (`x86_64-pc-windows-msvc`)

4. **Create GitHub Release**
   - Collect all artifacts
   - Generate release notes
   - Upload versioned and generic files

## Release Artifacts

After a successful build, the following files are available:

| File | Description |
|------|-------------|
| `kyco-vscode.vsix` | VS Code extension (generic) |
| `kyco-0.4.0.vsix` | VS Code extension (versioned) |
| `kyco-jetbrains.zip` | JetBrains plugin (generic) |
| `kyco-jetbrains-0.4.0.zip` | JetBrains plugin (versioned) |
| `kyco-macos-x64` | macOS Intel binary |
| `kyco-macos-arm64` | macOS Apple Silicon binary |
| `kyco-linux-x64` | Linux x64 binary |
| `kyco-windows-x64.exe` | Windows x64 binary |

## Download URLs

### Latest Release (always up-to-date)

```
https://github.com/MAF2414/kyco/releases/latest/download/kyco-vscode.vsix
https://github.com/MAF2414/kyco/releases/latest/download/kyco-jetbrains.zip
https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-arm64
https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-x64
https://github.com/MAF2414/kyco/releases/latest/download/kyco-linux-x64
https://github.com/MAF2414/kyco/releases/latest/download/kyco-windows-x64.exe
```

### Specific Version

```
https://github.com/MAF2414/kyco/releases/download/v0.4.0/kyco-0.4.0.vsix
```

## Installation Commands

### VS Code Extension
```bash
# Download and install latest
curl -L https://github.com/MAF2414/kyco/releases/latest/download/kyco-vscode.vsix -o kyco-vscode.vsix
code --install-extension kyco-vscode.vsix
```

### JetBrains Plugin
1. Download from GitHub releases
2. In IDE: Settings → Plugins → Gear icon → Install Plugin from Disk
3. Select the downloaded `.zip` file
4. Restart IDE

### Kyco App (macOS ARM)
```bash
curl -L https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-arm64 -o kyco
chmod +x kyco
./kyco
```

## Auto-Update Notifications

The Kyco GUI application automatically checks for updates on startup:

1. Queries GitHub API for latest release
2. Compares with current version (`CARGO_PKG_VERSION`)
3. Shows notification in status bar if update available
4. Click notification to open release page in browser

The update check runs in a background thread and does not block the application.

## Troubleshooting

### Build Failures

Check the GitHub Actions workflow logs:
1. Go to repository → Actions tab
2. Find the failed workflow run
3. Expand the failed job to see error details

### Common Issues

- **Missing dependencies**: Ensure all CI dependencies are installed
- **Version mismatch**: Run `bump-version.sh` to synchronize all versions
- **Tag already exists**: Delete the tag (`git tag -d v0.4.0 && git push origin :v0.4.0`) and retry

### Manual Build (if CI fails)

```bash
# Rust binary
cargo build --release

# VS Code extension
cd vscode-extension && npm install && npm run compile && npx @vscode/vsce package

# JetBrains plugin
cd jetbrains-plugin && ./gradlew buildPlugin
```

## Optional: Marketplace Publishing

### VS Code Marketplace (not currently used)

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
