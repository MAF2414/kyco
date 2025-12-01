# Kyco JetBrains Plugin

A minimal JetBrains plugin that sends code selections to a local Kyco server.

## Features

- Send current file path, selected text, line numbers, and workspace path to http://localhost:9876/selection
- Keyboard shortcut: `Ctrl+Alt+Y` (Windows/Linux) / `Ctrl+Cmd+Y` (Mac)
- Available in Tools menu and editor context menu

## Installation

### From Source

1. Navigate to the plugin directory:
   ```bash
   cd /path/to/CodeRail/jetbrains-plugin
   ```

2. Build the plugin:
   ```bash
   ./gradlew buildPlugin
   ```

3. The plugin ZIP file will be created in `build/distributions/`

4. Install in your JetBrains IDE:
   - Open Settings/Preferences
   - Go to Plugins
   - Click the gear icon and select "Install Plugin from Disk..."
   - Select the generated ZIP file from `build/distributions/`
   - Restart the IDE

### Development

To run the plugin in a development IDE instance:
```bash
./gradlew runIde
```

## Usage

1. Ensure your Kyco server is running on http://localhost:9876
2. Open any file in your JetBrains IDE
3. Select some text (optional - can send with empty selection)
4. Press `Ctrl+Alt+Y` (Windows/Linux) or `Ctrl+Cmd+Y` (Mac)
5. A notification will confirm success or show an error

## Data Format

The plugin sends a POST request with the following JSON payload:

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

- JetBrains IDE 2024.1 or higher (IntelliJ IDEA, WebStorm, PyCharm, etc.)
- Kyco server running on http://localhost:9876
- Java 17 or higher

## Supported IDEs

This plugin works with all JetBrains IDEs:
- IntelliJ IDEA (Community & Ultimate)
- WebStorm
- PyCharm
- PhpStorm
- GoLand
- RubyMine
- CLion
- Rider
- Android Studio

## License

See main project license.
