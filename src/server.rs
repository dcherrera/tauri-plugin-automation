//! HTTP server for automation commands

use std::io::Read;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tiny_http::{Server, Response, Header, Method};

const PORT: u16 = 9876;

// Global screenshot buffer (simple approach without lazy_static)
static SCREENSHOT_DATA: Mutex<Option<String>> = Mutex::new(None);

pub fn set_screenshot_data(data: String) {
    if let Ok(mut guard) = SCREENSHOT_DATA.lock() {
        *guard = Some(data);
    }
}

pub fn take_screenshot_data() -> Option<String> {
    if let Ok(mut guard) = SCREENSHOT_DATA.lock() {
        guard.take()
    } else {
        None
    }
}

/// Start the HTTP server
pub fn start_server(app_handle: AppHandle<tauri::Wry>) {
    let addr = format!("127.0.0.1:{}", PORT);

    let server = match Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Automation] Failed to start server on {}: {}", addr, e);
            return;
        }
    };

    println!("[Automation] HTTP server listening on http://{}", addr);

    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();

        println!("[Automation] {} {}", method, url);

        let response = match (&method, url.as_str()) {
            // Health check
            (&Method::Get, "/automation/health") => {
                json_response(serde_json::json!({
                    "status": "ok",
                    "port": PORT,
                    "version": "1.0.0"
                }))
            }

            // Execute command
            (&Method::Post, "/automation/execute") => {
                handle_execute(&app_handle, &mut request)
            }

            // Screenshot
            (&Method::Get, "/automation/screenshot") => {
                handle_screenshot(&app_handle)
            }

            // CORS preflight
            (&Method::Options, _) => {
                cors_response()
            }

            // 404
            _ => {
                json_response_with_status(
                    serde_json::json!({ "error": "Not found" }),
                    404
                )
            }
        };

        if let Err(e) = request.respond(response) {
            eprintln!("[Automation] Failed to send response: {}", e);
        }
    }
}

/// Handle execute command request
fn handle_execute(
    app_handle: &AppHandle<tauri::Wry>,
    request: &mut tiny_http::Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    // Read body
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        return json_response_with_status(
            serde_json::json!({ "error": format!("Failed to read body: {}", e) }),
            400
        );
    }

    // Parse JSON
    let payload: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return json_response_with_status(
                serde_json::json!({ "error": format!("Invalid JSON: {}", e) }),
                400
            );
        }
    };

    let command = match payload.get("command").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => {
            return json_response_with_status(
                serde_json::json!({ "error": "Missing 'command' field" }),
                400
            );
        }
    };

    let args = payload.get("args").cloned().unwrap_or(serde_json::json!({}));

    // Get the main window
    let window = match app_handle.get_window("main") {
        Some(w) => w,
        None => {
            return json_response_with_status(
                serde_json::json!({ "error": "Main window not found" }),
                500
            );
        }
    };

    // Build JavaScript to execute
    let args_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
    let script = format!(
        r#"
        (async function() {{
            if (typeof window.__TAURI_AUTOMATION__ === 'undefined') {{
                console.error('[Automation] Not initialized');
                return;
            }}
            try {{
                const result = await window.__TAURI_AUTOMATION__.execute('{}', {});
                window.__TAURI_AUTOMATION__._lastResult = {{ success: true, result: result }};
            }} catch (e) {{
                window.__TAURI_AUTOMATION__._lastResult = {{ success: false, error: e.message || String(e) }};
            }}
        }})();
        "#,
        command,
        args_json
    );

    // Execute the script
    if let Err(e) = window.eval(&script) {
        return json_response_with_status(
            serde_json::json!({ "error": format!("Script execution failed: {}", e) }),
            500
        );
    }

    // Wait a bit for async commands to complete
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Return success - the result is stored in the webview for debugging
    json_response(serde_json::json!({
        "success": true,
        "message": "Command executed",
        "command": command
    }))
}

/// Handle screenshot request
fn handle_screenshot(app_handle: &AppHandle<tauri::Wry>) -> Response<std::io::Cursor<Vec<u8>>> {
    let window = match app_handle.get_window("main") {
        Some(w) => w,
        None => {
            return json_response_with_status(
                serde_json::json!({ "error": "Main window not found" }),
                500
            );
        }
    };

    // Request screenshot from JS
    let script = r#"
        (async function() {
            if (typeof window.__TAURI_AUTOMATION__ === 'undefined') {
                console.error('[Automation] Not initialized');
                return;
            }
            try {
                await window.__TAURI_AUTOMATION__.captureAndSend();
            } catch (e) {
                console.error('[Automation] Screenshot failed:', e);
            }
        })();
    "#;

    if let Err(e) = window.eval(script) {
        return json_response_with_status(
            serde_json::json!({ "error": format!("Screenshot request failed: {}", e) }),
            500
        );
    }

    // Wait for JS to send the screenshot data
    // html2canvas can take a while, especially on first load
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // Check if we have screenshot data
    if let Some(data_url) = take_screenshot_data() {
        // Parse data URL: data:image/png;base64,....
        if let Some(base64_data) = data_url.strip_prefix("data:image/png;base64,") {
            match base64_decode(base64_data) {
                Ok(bytes) => {
                    return png_response(bytes);
                }
                Err(e) => {
                    return json_response_with_status(
                        serde_json::json!({ "error": format!("Base64 decode failed: {}", e) }),
                        500
                    );
                }
            }
        }
    }

    json_response_with_status(
        serde_json::json!({ "error": "Screenshot not available. Make sure html2canvas is loaded." }),
        500
    )
}

/// Simple base64 decoder
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim();
    let chars: Vec<char> = input.chars().filter(|c| !c.is_whitespace()).collect();

    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut output = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits_collected = 0;

    for c in chars {
        if c == '=' {
            break;
        }

        let value = ALPHABET.iter().position(|&x| x == c as u8)
            .ok_or_else(|| format!("Invalid base64 character: {}", c))? as u32;

        buffer = (buffer << 6) | value;
        bits_collected += 6;

        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }

    Ok(output)
}

/// Create a JSON response
fn json_response(data: serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response_with_status(data, 200)
}

/// Create a JSON response with status
fn json_response_with_status(data: serde_json::Value, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(&data).unwrap_or_else(|_| b"{}".to_vec());
    let len = body.len();
    let cursor = std::io::Cursor::new(body);

    Response::new(
        tiny_http::StatusCode(status),
        vec![
            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap(),
        ],
        cursor,
        Some(len),
        None,
    )
}

/// Create a PNG response
fn png_response(data: Vec<u8>) -> Response<std::io::Cursor<Vec<u8>>> {
    let len = data.len();
    let cursor = std::io::Cursor::new(data);

    Response::new(
        tiny_http::StatusCode(200),
        vec![
            Header::from_bytes(&b"Content-Type"[..], &b"image/png"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        ],
        cursor,
        Some(len),
        None,
    )
}

/// CORS preflight response
fn cors_response() -> Response<std::io::Cursor<Vec<u8>>> {
    let cursor = std::io::Cursor::new(Vec::new());

    Response::new(
        tiny_http::StatusCode(204),
        vec![
            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap(),
        ],
        cursor,
        Some(0),
        None,
    )
}
