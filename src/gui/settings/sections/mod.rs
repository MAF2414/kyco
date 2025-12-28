//! Settings section render functions
//!
//! Each function renders a distinct section of the settings panel.

mod general;
mod http_server;
mod ide_extensions;
mod orchestrator;
mod output_schema;
mod voice;

pub use general::render_settings_general;
pub use http_server::render_settings_http_server;
pub use ide_extensions::render_settings_ide_extensions;
pub use orchestrator::render_settings_orchestrator;
pub use output_schema::render_settings_output_schema;
pub use voice::render_settings_voice;
