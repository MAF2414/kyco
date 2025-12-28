//! Update installation logic

use std::io::Read;

use super::UpdateInfo;

/// Check if we have write permission to the executable's directory
fn has_write_permission(path: &std::path::Path) -> bool {
    if let Some(parent) = path.parent() {
        // Try to create a temp file to check write permission
        let test_path = parent.join(".kyco_write_test");
        if std::fs::write(&test_path, "test").is_ok() {
            let _ = std::fs::remove_file(&test_path);
            return true;
        }
    }
    false
}

/// Download and install update, returns status message
pub fn install_update(info: &UpdateInfo) -> Result<String, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;

    // Check if we need elevated privileges
    if !has_write_permission(&current_exe) {
        #[cfg(target_os = "macos")]
        {
            return install_update_with_admin_macos(info, &current_exe);
        }
        #[cfg(not(target_os = "macos"))]
        {
            return Err("Permission denied. Please run as administrator or move kyco to a user-writable location.".to_string());
        }
    }

    install_update_direct(info, &current_exe)
}

/// Install update directly (when we have write permissions)
fn install_update_direct(
    info: &UpdateInfo,
    current_exe: &std::path::Path,
) -> Result<String, String> {
    let response = ureq::get(&info.download_url)
        .set("User-Agent", "kyco-update-installer")
        .call()
        .map_err(|e| format!("Failed to download update: {}", e))?;

    let mut binary_data = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut binary_data)
        .map_err(|e| format!("Failed to read download: {}", e))?;

    let backup_path = current_exe.with_extension("backup");
    if backup_path.exists() {
        std::fs::remove_file(&backup_path)
            .map_err(|e| format!("Failed to remove old backup: {}", e))?;
    }
    std::fs::rename(current_exe, &backup_path)
        .map_err(|e| format!("Failed to backup current binary: {}", e))?;

    std::fs::write(current_exe, &binary_data).map_err(|e| {
        // Try to restore backup on failure
        let _ = std::fs::rename(&backup_path, current_exe);
        format!("Failed to write new binary: {}", e)
    })?;

    // Set executable permissions (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(current_exe)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(current_exe, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    let _ = std::fs::remove_file(&backup_path);

    Ok(format!(
        "Updated to v{}! Please restart kyco.",
        info.version
    ))
}

/// Install update using macOS admin privileges via osascript
#[cfg(target_os = "macos")]
fn install_update_with_admin_macos(
    info: &UpdateInfo,
    current_exe: &std::path::Path,
) -> Result<String, String> {
    use std::process::Command;

    let exe_path = current_exe.to_string_lossy();
    let download_url = &info.download_url;

    // Create a shell script that will be run with admin privileges
    // This script: downloads the binary, backs up the current one, replaces it, and sets permissions
    let script = format!(
        r#"
        set -e
        TEMP_FILE=$(mktemp)
        curl -sL "{download_url}" -o "$TEMP_FILE"
        if [ -f "{exe_path}.backup" ]; then rm "{exe_path}.backup"; fi
        mv "{exe_path}" "{exe_path}.backup"
        mv "$TEMP_FILE" "{exe_path}"
        chmod 755 "{exe_path}"
        rm -f "{exe_path}.backup"
        "#,
        download_url = download_url,
        exe_path = exe_path
    );

    // Use osascript to run the script with administrator privileges
    // This will show the native macOS password dialog
    let output = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"do shell script "{}" with administrator privileges"#,
            script
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', " ")
        ))
        .output()
        .map_err(|e| format!("Failed to request admin privileges: {}", e))?;

    if output.status.success() {
        Ok(format!(
            "Updated to v{}! Please restart kyco.",
            info.version
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("(-128)") {
            Err("Update cancelled by user.".to_string())
        } else {
            Err(format!("Update failed: {}", stderr))
        }
    }
}

/// Open a URL in the default browser
pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
}
