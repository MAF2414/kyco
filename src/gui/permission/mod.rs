//! Permission popup for tool approval requests
//!
//! When Claude needs permission to use a tool (in `default` or `acceptEdits` mode),
//! this popup is shown to the user to approve or deny the request.

mod render;
mod types;

pub use render::render_permission_popup;
pub use types::{PermissionAction, PermissionDecision, PermissionPopupState, PermissionRequest};
