#!/bin/bash
# Quick setup script for Kyco VS Code Extension

set -e

echo "Installing dependencies..."
npm install

echo "Compiling TypeScript..."
npm run compile

echo ""
echo "Setup complete! To install the extension:"
echo ""
echo "Option 1 - Development Mode:"
echo "  1. Open this folder in VS Code"
echo "  2. Press F5 to launch Extension Development Host"
echo ""
echo "Option 2 - Install Locally:"
echo "  Run: code --install-extension kyco-0.1.0.vsix"
echo "  (after running: npm install -g @vscode/vsce && vsce package)"
echo ""
echo "Option 3 - Manual Install:"
echo "  Copy this folder to ~/.vscode/extensions/kyco-0.1.0/"
echo "  Then reload VS Code"
