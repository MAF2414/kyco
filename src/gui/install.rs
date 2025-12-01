//! IDE extension installation module
//!
//! This module handles installation of IDE extensions (VS Code, JetBrains, etc.)
//! that integrate with kyco for sending code selections.

use std::path::Path;
use std::process::Command;

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
    let extension_dir = work_dir.join("vscode-extension");

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
