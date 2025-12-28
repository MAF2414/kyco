//! Shell and AppleScript helper utilities.

use std::process::Command;

/// Check if a process with the given PID is still running.
///
/// Uses `kill -0` which sends no signal but checks process existence.
/// This is a POSIX-standard way to verify a process is alive.
///
/// # Arguments
///
/// * `pid` - The process ID to check
///
/// # Returns
///
/// `true` if the process exists and is accessible, `false` otherwise.
pub fn is_process_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Escape a string for safe shell use.
///
/// Wraps the string in single quotes and escapes embedded single quotes
/// using the `'\''` technique (end quote, escaped quote, start quote).
///
/// # Example
///
/// ```ignore
/// assert_eq!(shell_escape("hello"), "'hello'");
/// assert_eq!(shell_escape("it's"), "'it'\\''s'");
/// ```
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Quote a string for AppleScript embedding.
///
/// Escapes backslashes and double quotes, then wraps in double quotes.
/// Equivalent to AppleScript's `quoted form of` for string literals.
///
/// # Arguments
///
/// * `s` - The string to quote
///
/// # Returns
///
/// A properly escaped string safe for AppleScript interpolation.
#[allow(dead_code)]
pub fn applescript_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }
}
