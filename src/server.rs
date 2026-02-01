//! HTTP server for automation commands
//!
//! Provides HTTP API on port 9876 for external automation.

use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Response, Server};

use crate::take_screenshot_data;

const PORT: u16 = 9876;

/// Run the HTTP automation server
pub fn run_server(app_handle: AppHandle) {
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
            (&Method::Get, "/automation/health") => json_response(serde_json::json!({
                "status": "ok",
                "port": PORT,
                "version": "0.2.0"
            })),

            (&Method::Post, "/automation/execute") => handle_execute(&app_handle, &mut request),

            (&Method::Get, "/automation/screenshot") => handle_screenshot(&app_handle),

            (&Method::Options, _) => cors_response(),

            _ => json_response_with_status(serde_json::json!({ "error": "Not found" }), 404),
        };

        if let Err(e) = request.respond(response) {
            eprintln!("[Automation] Failed to send response: {}", e);
        }
    }
}

fn handle_execute(
    app_handle: &AppHandle,
    request: &mut tiny_http::Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        return json_response_with_status(
            serde_json::json!({ "error": format!("Failed to read body: {}", e) }),
            400,
        );
    }

    let payload: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return json_response_with_status(
                serde_json::json!({ "error": format!("Invalid JSON: {}", e) }),
                400,
            );
        }
    };

    let command = match payload.get("command").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => {
            return json_response_with_status(
                serde_json::json!({ "error": "Missing 'command' field" }),
                400,
            );
        }
    };

    let args = payload.get("args").cloned().unwrap_or(serde_json::json!({}));

    let window = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => {
            return json_response_with_status(
                serde_json::json!({ "error": "Main window not found" }),
                500,
            );
        }
    };

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
        command, args_json
    );

    if let Err(e) = window.eval(&script) {
        return json_response_with_status(
            serde_json::json!({ "error": format!("Script execution failed: {}", e) }),
            500,
        );
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    json_response(serde_json::json!({
        "success": true,
        "message": "Command executed",
        "command": command
    }))
}

fn handle_screenshot(app_handle: &AppHandle) -> Response<std::io::Cursor<Vec<u8>>> {
    let window = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => {
            return json_response_with_status(
                serde_json::json!({ "error": "Main window not found" }),
                500,
            );
        }
    };

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
            500,
        );
    }

    std::thread::sleep(std::time::Duration::from_millis(2000));

    if let Some(data_url) = take_screenshot_data() {
        if let Some(base64_data) = data_url.strip_prefix("data:image/png;base64,") {
            match base64_decode(base64_data) {
                Ok(bytes) => return png_response(bytes),
                Err(e) => {
                    return json_response_with_status(
                        serde_json::json!({ "error": format!("Base64 decode failed: {}", e) }),
                        500,
                    );
                }
            }
        }
    }

    json_response_with_status(
        serde_json::json!({ "error": "Screenshot not available. Make sure html2canvas is loaded." }),
        500,
    )
}

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

        let value = ALPHABET
            .iter()
            .position(|&x| x == c as u8)
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

fn json_response(data: serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response_with_status(data, 200)
}

fn json_response_with_status(
    data: serde_json::Value,
    status: u16,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(&data).unwrap_or_else(|_| b"{}".to_vec());
    let len = body.len();
    let cursor = std::io::Cursor::new(body);

    Response::new(
        tiny_http::StatusCode(status),
        vec![
            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..])
                .unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap(),
        ],
        cursor,
        Some(len),
        None,
    )
}

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

fn cors_response() -> Response<std::io::Cursor<Vec<u8>>> {
    let cursor = std::io::Cursor::new(Vec::new());

    Response::new(
        tiny_http::StatusCode(204),
        vec![
            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..])
                .unwrap(),
            Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap(),
        ],
        cursor,
        Some(0),
        None,
    )
}
