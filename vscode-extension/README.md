# Kyco VS Code Extension

A minimal VS Code extension that sends code selections to a local Kyco server.

## Features

- Send current file path, selected text, line numbers, and workspace path to http://localhost:9876/selection
- Keyboard shortcut: `Cmd+Option+K` (Mac) / `Ctrl+Alt+K` (Windows/Linux)
- Command palette: "Kyco: Send Selection"

## Installation

### From Source

1. Navigate to the extension directory:
   ```bash
   cd /Users/maltefries/Repo/CodeRail/vscode-extension
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Compile the extension:
   ```bash
   npm run compile
   ```

4. Install the extension locally:
   - Press `F5` in VS Code to open a new Extension Development Host window, OR
   - Copy the entire `vscode-extension` folder to your VS Code extensions directory:
     - Mac/Linux: `~/.vscode/extensions/kyco-0.1.0/`
     - Windows: `%USERPROFILE%\.vscode\extensions\kyco-0.1.0\`
   - Then reload VS Code

### Package as VSIX (Optional)

To create a distributable `.vsix` file:

1. Install vsce:
   ```bash
   npm install -g @vscode/vsce
   ```

2. Package the extension:
   ```bash
   vsce package
   ```

3. Install the generated `.vsix` file:
   - Open VS Code
   - Go to Extensions view (`Cmd+Shift+X` / `Ctrl+Shift+X`)
   - Click the `...` menu at the top
   - Select "Install from VSIX..."
   - Choose the generated `kyco-0.1.0.vsix` file

## Usage

1. Ensure your Kyco server is running on http://localhost:9876
2. Open any file in VS Code
3. Select some text (optional - can send with empty selection)
4. Press `Cmd+Option+K` (Mac) or `Ctrl+Alt+K` (Windows/Linux)
5. A notification will confirm success or show an error

## Data Format

The extension sends a POST request with the following JSON payload:

```json
{
  "file_path": "/absolute/path/to/file.rs",
  "selected_text": "the selected code",
  "line_start": 42,
  "line_end": 45,
  "workspace": "/absolute/path/to/workspace"
}
```

## Requirements

- VS Code 1.85.0 or higher
- Kyco server running on http://localhost:9876

## Development

### Watch Mode

Run TypeScript compiler in watch mode:
```bash
npm run watch
```

Then press `F5` to start debugging in the Extension Development Host.

### Debugging

1. Open the extension folder in VS Code
2. Press `F5` to start debugging
3. A new VS Code window will open with the extension loaded
4. Set breakpoints in the TypeScript source files
5. Trigger the command to test

## License

See main project license.
