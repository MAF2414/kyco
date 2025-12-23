//! Selection module for the GUI
//!
//! Handles all selection-related functionality including:
//! - Selection context (information from IDE extensions)
//! - Autocomplete suggestions for modes and agents
//! - Selection popup UI

pub mod autocomplete;
mod context;
mod popup;

pub use autocomplete::{AutocompleteState, Suggestion};
pub use context::SelectionContext;
pub use popup::{
    BatchPopupState, SelectionPopupAction, SelectionPopupState, render_batch_popup,
    render_selection_popup,
};
