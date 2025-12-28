//! Types for selection and batch popups

use super::super::autocomplete::Suggestion;
use super::super::context::SelectionContext;
use crate::gui::http_server::BatchFile;
use crate::gui::voice::{VoiceInputMode, VoiceState};

/// Actions that can be triggered from the selection popup
#[derive(Debug, Clone)]
pub enum SelectionPopupAction {
    /// User changed the input text
    InputChanged,
    /// User clicked a suggestion
    SuggestionClicked(usize),
    /// User toggled voice recording
    ToggleRecording,
}

/// State required for rendering the selection popup
pub struct SelectionPopupState<'a> {
    pub selection: &'a SelectionContext,
    pub popup_input: &'a mut String,
    pub popup_status: &'a Option<(String, bool)>,
    pub suggestions: &'a [Suggestion],
    pub selected_suggestion: usize,
    pub show_suggestions: bool,
    pub cursor_to_end: &'a mut bool,
    pub voice_state: VoiceState,
    pub voice_mode: VoiceInputMode,
    pub voice_last_error: Option<&'a str>,
}

/// State required for rendering the batch popup
pub struct BatchPopupState<'a> {
    pub batch_files: &'a [BatchFile],
    pub popup_input: &'a mut String,
    pub popup_status: &'a Option<(String, bool)>,
    pub suggestions: &'a [Suggestion],
    pub selected_suggestion: usize,
    pub show_suggestions: bool,
    pub cursor_to_end: &'a mut bool,
}

/// Result from rendering the input field
pub(crate) struct InputFieldResult {
    /// True if the text input changed
    pub input_changed: bool,
    /// True if the microphone button was clicked
    pub mic_clicked: bool,
}

/// Maximum number of suggestions to display
pub(crate) const MAX_SUGGESTIONS_VISIBLE: usize = 5;
