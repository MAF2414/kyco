//! IDE extension installation module
//!
//! This module handles installation of IDE extensions (VS Code, JetBrains, etc.)
//! that integrate with kyco for sending code selections.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the kyco installation directory (where the executable lives)
///
/// This is used to find bundled resources like IDE extensions.
/// Falls back to the provided work_dir if the executable path cannot be determined.
fn get_kyco_install_dir(work_dir: &Path) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        // If exe is in a bin/ directory, go up one level to find extensions
        .map(|dir| {
            if dir.ends_with("bin") || dir.ends_with("debug") || dir.ends_with("release") {
                dir.parent().map(|p| p.to_path_buf()).unwrap_or(dir)
            } else {
                dir
            }
        })
        .unwrap_or_else(|| work_dir.to_path_buf())
}

/// Result of an extension installation operation
pub struct ExtensionInstallResult {
    /// Human-readable message describing the result
    pub message: String,
    /// Whether the installation encountered an error
    pub is_error: bool,
}

impl ExtensionInstallResult {
    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
        }
    }
}

/// Install the VS Code extension from the vscode-extension directory
///
/// This function:
/// 1. Runs `npm install` to install dependencies
/// 2. Compiles TypeScript with `npm run compile`
/// 3. Installs vsce if needed
/// 4. Packages the extension with vsce
/// 5. Installs the packaged .vsix file
pub fn install_vscode_extension(work_dir: &Path) -> ExtensionInstallResult {
    let install_dir = get_kyco_install_dir(work_dir);
    let extension_dir = install_dir.join("vscode-extension");

    if !extension_dir.exists() {
        return ExtensionInstallResult::error(format!(
            "Extension not found at: {}",
            extension_dir.display()
        ));
    }

    // Check if npm is available
    if Command::new("npm").arg("--version").output().is_err() {
        return ExtensionInstallResult::error("npm not found. Please install Node.js first.");
    }

    // Step 1: npm install
    if let Err(e) = run_npm_install(&extension_dir) {
        return ExtensionInstallResult::error(format!("npm install failed: {}", e));
    }

    // Step 2: Compile TypeScript
    if let Err(e) = run_npm_compile(&extension_dir) {
        return ExtensionInstallResult::error(format!("Compile failed: {}", e));
    }

    // Step 3: Ensure vsce is installed
    if let Err(e) = ensure_vsce_installed(&extension_dir) {
        return ExtensionInstallResult::error(format!("Failed to install vsce: {}", e));
    }

    // Step 4: Package the extension
    match package_extension(&extension_dir) {
        Ok(vsix_path) => {
            // Step 5: Install the extension
            install_vsix(&vsix_path)
        }
        Err(e) => ExtensionInstallResult::error(format!("Packaging failed: {}", e)),
    }
}

/// Run npm install in the extension directory
fn run_npm_install(extension_dir: &Path) -> Result<(), String> {
    Command::new("npm")
        .arg("install")
        .current_dir(extension_dir)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Run npm compile in the extension directory
fn run_npm_compile(extension_dir: &Path) -> Result<(), String> {
    Command::new("npm")
        .args(["run", "compile"])
        .current_dir(extension_dir)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Ensure vsce is installed (install if needed)
fn ensure_vsce_installed(extension_dir: &Path) -> Result<(), String> {
    let vsce_check = Command::new("npx")
        .args(["vsce", "--version"])
        .current_dir(extension_dir)
        .output();

    let needs_install = match vsce_check {
        Ok(output) => !output.status.success(),
        Err(_) => true,
    };

    if needs_install {
        Command::new("npm")
            .args(["install", "--save-dev", "@vscode/vsce"])
            .current_dir(extension_dir)
            .output()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Package the extension and return the path to the .vsix file
fn package_extension(extension_dir: &Path) -> Result<std::path::PathBuf, String> {
    let output = Command::new("npx")
        .args(["vsce", "package", "--allow-missing-repository"])
        .current_dir(extension_dir)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let vsix_path = extension_dir.join("kyco-0.1.0.vsix");
    if !vsix_path.exists() {
        return Err("Package created but .vsix file not found.".to_string());
    }

    Ok(vsix_path)
}

/// Install a .vsix file using the VS Code CLI
fn install_vsix(vsix_path: &Path) -> ExtensionInstallResult {
    let install_result = Command::new("code")
        .args([
            "--install-extension",
            vsix_path.to_str().unwrap_or("kyco-0.1.0.vsix"),
        ])
        .output();

    match install_result {
        Ok(output) if output.status.success() => ExtensionInstallResult::success(
            "VS Code extension installed! Restart VS Code to activate.\nHotkey: Cmd+Option+K",
        ),
        Ok(_) => ExtensionInstallResult::success(format!(
            "Extension packaged! Install manually:\ncode --install-extension {}",
            vsix_path.display()
        )),
        Err(_) => ExtensionInstallResult::success(format!(
            "Extension packaged! VS Code CLI not found.\nRun: code --install-extension {}",
            vsix_path.display()
        )),
    }
}

/// Install the JetBrains plugin from the jetbrains-plugin directory
///
/// This function:
/// 1. Runs `./gradlew buildPlugin` to build the plugin
/// 2. Locates the built .zip file
/// 3. Provides instructions for manual installation
pub fn install_jetbrains_plugin(work_dir: &Path) -> ExtensionInstallResult {
    let install_dir = get_kyco_install_dir(work_dir);
    let plugin_dir = install_dir.join("jetbrains-plugin");

    if !plugin_dir.exists() {
        return ExtensionInstallResult::error(format!(
            "Plugin not found at: {}",
            plugin_dir.display()
        ));
    }

    // Check if gradlew exists
    let gradlew = plugin_dir.join("gradlew");
    if !gradlew.exists() {
        return ExtensionInstallResult::error("gradlew not found in jetbrains-plugin directory.");
    }

    // Build the plugin
    if let Err(e) = run_gradle_build(&plugin_dir) {
        return ExtensionInstallResult::error(format!("Gradle build failed: {}", e));
    }

    // Find the built plugin zip
    match find_plugin_zip(&plugin_dir) {
        Ok(zip_path) => ExtensionInstallResult::success(format!(
            "JetBrains plugin built successfully!\n\n\
            To install:\n\
            1. Open your JetBrains IDE (IntelliJ, WebStorm, etc.)\n\
            2. Go to Settings → Plugins → ⚙️ → Install Plugin from Disk\n\
            3. Select: {}\n\
            4. Restart the IDE\n\n\
            Hotkey: Ctrl+Alt+Y (Ctrl+Cmd+Y on Mac)",
            zip_path.display()
        )),
        Err(e) => ExtensionInstallResult::error(format!("Build completed but plugin not found: {}", e)),
    }
}

/// Run gradle build to create the plugin
fn run_gradle_build(plugin_dir: &Path) -> Result<(), String> {
    let gradlew = if cfg!(windows) { "gradlew.bat" } else { "./gradlew" };

    let output = Command::new(gradlew)
        .arg("buildPlugin")
        .current_dir(plugin_dir)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("{}\n{}", stdout, stderr));
    }

    Ok(())
}

/// Find the built plugin zip file in build/distributions
fn find_plugin_zip(plugin_dir: &Path) -> Result<std::path::PathBuf, String> {
    let dist_dir = plugin_dir.join("build").join("distributions");

    if !dist_dir.exists() {
        return Err("build/distributions directory not found".to_string());
    }

    // Look for any .zip file
    let entries = std::fs::read_dir(&dist_dir)
        .map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "zip") {
            return Ok(path);
        }
    }

    Err("No .zip file found in build/distributions".to_string())
}
