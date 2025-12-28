//! Font configuration for the GUI

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use tracing::info;

/// Configure fonts with system fallbacks for Unicode symbols and emojis
pub(super) fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Platform-specific font configurations
    // Each entry: (name, path) - will be tried in order
    #[cfg(target_os = "macos")]
    let font_fallbacks: &[(&str, &str)] = &[
        ("symbols", "/System/Library/Fonts/Apple Symbols.ttf"),
        (
            "arial_unicode",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ),
    ];

    #[cfg(target_os = "windows")]
    let font_fallbacks: &[(&str, &str)] = &[
        ("symbols", "C:\\Windows\\Fonts\\seguisym.ttf"),
        ("segoe", "C:\\Windows\\Fonts\\segoeui.ttf"),
    ];

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let font_fallbacks: &[(&str, &str)] = &[
        (
            "symbols",
            "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
        ),
        (
            "symbols_alt",
            "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
        ),
        ("dejavu", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ];

    for (name, path) in font_fallbacks {
        if let Ok(font_data) = std::fs::read(path) {
            fonts
                .font_data
                .insert((*name).to_owned(), FontData::from_owned(font_data).into());

            // Add as fallback for both font families
            if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
                family.push((*name).to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
                family.push((*name).to_owned());
            }

            info!("[kyco] Loaded fallback font '{}' from: {}", name, path);
        }
    }

    ctx.set_fonts(fonts);
}
