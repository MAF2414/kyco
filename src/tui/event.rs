//! Event handling for the TUI

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// A key was pressed
    Key(KeyEvent),

    /// Terminal was resized
    #[allow(dead_code)]
    Resize(u16, u16),

    /// No event (tick)
    Tick,
}

/// Event handler for the TUI
pub struct EventHandler {
    /// Tick rate for polling
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Poll for the next event
    pub fn next(&self) -> anyhow::Result<AppEvent> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => Ok(AppEvent::Key(key)),
                Event::Resize(w, h) => Ok(AppEvent::Resize(w, h)),
                _ => Ok(AppEvent::Tick),
            }
        } else {
            Ok(AppEvent::Tick)
        }
    }
}

/// Check if a key event matches Ctrl+C or 'q' for quit
pub fn is_quit_event(key: &KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } | KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            ..
        }
    )
}
