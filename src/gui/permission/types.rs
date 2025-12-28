//! Types for permission popup requests and actions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A pending permission request from the Bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    /// Unique request ID (must be sent back with response)
    pub request_id: String,
    /// Session/job ID this request belongs to
    pub session_id: String,
    /// Tool name (e.g., "Bash", "Write", "Edit")
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: HashMap<String, serde_json::Value>,
    /// Timestamp when request was received
    pub received_at: u64,
}

/// User's decision on a permission request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

/// Actions that can be triggered from the permission popup
#[derive(Debug, Clone)]
pub enum PermissionAction {
    /// User approved the request
    Approve(String), // request_id
    /// User approved this and all pending requests
    ApproveAll(Vec<String>), // request_ids
    /// User denied the request
    Deny(String, String), // request_id, reason
    /// User dismissed the popup (treat as deny)
    Dismiss(String), // request_id
}

/// State for the permission popup
pub struct PermissionPopupState {
    /// Currently displayed permission request
    pub current_request: Option<PermissionRequest>,
    /// Queue of pending requests (if multiple)
    pub pending_requests: Vec<PermissionRequest>,
    /// Whether the popup is visible
    pub visible: bool,
    /// Whether we should bring app to foreground
    pub should_focus: bool,
}

impl Default for PermissionPopupState {
    fn default() -> Self {
        Self {
            current_request: None,
            pending_requests: Vec::new(),
            visible: false,
            should_focus: false,
        }
    }
}

impl PermissionPopupState {
    /// Add a new permission request
    pub fn add_request(&mut self, request: PermissionRequest) {
        if self.current_request.is_none() {
            self.current_request = Some(request);
            self.visible = true;
            self.should_focus = true;
        } else {
            self.pending_requests.push(request);
        }
    }

    /// Move to the next request (after handling current one)
    pub fn next_request(&mut self) {
        self.current_request = self.pending_requests.pop();
        self.visible = self.current_request.is_some();
        self.should_focus = self.visible;
    }

    /// Get total pending count (including current)
    pub fn pending_count(&self) -> usize {
        let current = if self.current_request.is_some() { 1 } else { 0 };
        current + self.pending_requests.len()
    }
}
