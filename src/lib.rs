//! Tauri Automation Plugin
//!
//! Provides HTTP API for external automation of the application.
//! Enables Claude Code and other tools to control Tauri apps for testing.
//!
//! ## Usage
//!
//! ```rust,ignore
//! tauri::Builder::default()
//!     .setup(|app| {
//!         tauri_plugin_automation_server::start_server(app.handle().clone());
//!         Ok(())
//!     })
//!     .invoke_handler(tauri::generate_handler![
//!         tauri_plugin_automation_server::automation_screenshot_data,
//!     ])
//! ```

pub mod server;

use std::sync::Mutex;
use tauri::AppHandle;

/// Global screenshot data buffer
static SCREENSHOT_DATA: Mutex<Option<String>> = Mutex::new(None);

/// Store screenshot data from JavaScript
pub fn set_screenshot_data(data: String) {
    if let Ok(mut guard) = SCREENSHOT_DATA.lock() {
        *guard = Some(data);
    }
}

/// Take screenshot data (clears buffer)
pub fn take_screenshot_data() -> Option<String> {
    if let Ok(mut guard) = SCREENSHOT_DATA.lock() {
        guard.take()
    } else {
        None
    }
}

/// Start the automation HTTP server
///
/// Call this in your Tauri setup to enable external automation.
///
/// # Example
/// ```rust,ignore
/// tauri::Builder::default()
///     .setup(|app| {
///         tauri_plugin_automation_server::start_server(app.handle().clone());
///         Ok(())
///     })
/// ```
pub fn start_server(app_handle: AppHandle) {
    std::thread::spawn(move || {
        server::run_server(app_handle);
    });
    println!("[Automation] Plugin initialized - HTTP server starting on port 9876");
}

/// Receive screenshot data from JavaScript
///
/// Users should create their own command wrapper:
/// ```rust,ignore
/// #[tauri::command]
/// fn automation_screenshot_data(data: String) {
///     tauri_plugin_automation_server::set_screenshot_data(data);
/// }
/// ```
pub fn receive_screenshot(data: String) {
    set_screenshot_data(data);
}
