//! File system watcher for detecting code changes

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Events from the file watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A file was modified or created
    FileChanged(PathBuf),
    /// An error occurred
    Error(String),
}

/// File system watcher that emits events when source files change
pub struct FileWatcher {
    /// Channel to receive watch events
    rx: mpsc::Receiver<WatchEvent>,
    /// The watcher itself (kept alive)
    _watcher: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl FileWatcher {
    /// Create a new file watcher for the given directory
    pub fn new(root: &Path, debounce_ms: u64) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let tx_clone = tx.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(debounce_ms),
            move |res: DebounceEventResult| {
                match res {
                    Ok(events) => {
                        for event in events {
                            // Only care about file modifications
                            if matches!(event.kind, DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous) {
                                // Filter to only source files we care about
                                if Self::is_source_file(&event.path) {
                                    let _ = tx_clone.send(WatchEvent::FileChanged(event.path));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx_clone.send(WatchEvent::Error(e.to_string()));
                    }
                }
            },
        )?;

        // Watch the directory recursively
        debouncer.watcher().watch(root, notify::RecursiveMode::Recursive)?;

        Ok(Self {
            rx,
            _watcher: debouncer,
        })
    }

    /// Try to receive a watch event (non-blocking)
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.rx.try_recv().ok()
    }

    /// Check if a file is a source file we should watch
    fn is_source_file(path: &Path) -> bool {
        // Skip hidden files and directories (starting with '.')
        if path.components().any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
        }) {
            return false;
        }

        // Skip kyco config file
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name == "kyco.toml" {
                return false;
            }
        }

        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            // Files without extension (Makefile, Dockerfile, etc.)
            return true;
        };

        let ext_lower = ext.to_lowercase();

        // Exclude known binary formats
        !matches!(
            ext_lower.as_str(),
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "svg" | "webp" | "tiff" | "psd"
            // Audio/Video
            | "mp3" | "mp4" | "wav" | "avi" | "mkv" | "mov" | "flac" | "ogg" | "webm"
            // Archives
            | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "iso"
            // Binaries
            | "exe" | "dll" | "so" | "dylib" | "bin" | "o" | "a" | "lib"
            // Documents (binary)
            | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt"
            // Fonts
            | "ttf" | "otf" | "woff" | "woff2" | "eot"
            // Database
            | "db" | "sqlite" | "sqlite3"
            // Lock files (usually auto-generated, large)
            | "lock"
            // Other binary
            | "class" | "pyc" | "pyo" | "wasm" | "rlib"
        )
    }
}
