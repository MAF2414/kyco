//! Selection popup component for the GUI
//!
//! Renders the selection popup that appears when code is selected in an IDE.
//! Allows the user to enter a mode and prompt to create a job.

mod batch;
mod selection;
mod types;
mod widgets;

// Re-export public API
pub use batch::render_batch_popup;
pub use selection::render_selection_popup;
pub use types::{BatchPopupState, SelectionPopupAction, SelectionPopupState};
