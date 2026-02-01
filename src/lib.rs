//! Tauri Automation Plugin
//!
//! Provides HTTP API for external automation of the application.
//! Enabled via the "automation" feature flag.

pub mod server;

use std::sync::{Arc, Mutex};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, Window,
};

/// Shared state for automation
pub struct AutomationState {
    pub window: Arc<Mutex<Option<Window<tauri::Wry>>>>,
}

impl Default for AutomationState {
    fn default() -> Self {
        Self {
            window: Arc::new(Mutex::new(None)),
        }
    }
}

/// Initialize the automation plugin
pub fn init() -> TauriPlugin<tauri::Wry> {
    Builder::new("automation")
        .setup(|app| {
            let state = AutomationState::default();
            app.manage(state);

            // Start HTTP server in background thread
            let app_handle = app.clone();
            std::thread::spawn(move || {
                server::start_server(app_handle);
            });

            println!("[Automation] Plugin initialized - HTTP server starting on port 9876");
            Ok(())
        })
        .build()
}

/// Execute an automation command via JavaScript evaluation
pub fn execute_command(window: &Window<tauri::Wry>, command: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let args_json = serde_json::to_string(args).map_err(|e| e.to_string())?;

    let script = format!(
        r#"
        (async function() {{
            try {{
                if (typeof window.__TAURI_AUTOMATION__ === 'undefined') {{
                    return JSON.stringify({{ error: 'Automation not initialized. Call initAutomation() first.' }});
                }}
                const result = await window.__TAURI_AUTOMATION__.execute('{}', {});
                return JSON.stringify({{ success: true, result: result }});
            }} catch (e) {{
                return JSON.stringify({{ error: e.message || String(e) }});
            }}
        }})()
        "#,
        command,
        args_json
    );

    // Execute JavaScript and get result
    let result = window.eval(&script);

    match result {
        Ok(_) => {
            // For Tauri 1.x, eval doesn't return a value directly
            Ok(serde_json::json!({ "success": true, "note": "Command sent to webview" }))
        }
        Err(e) => Err(format!("Failed to execute script: {}", e))
    }
}
