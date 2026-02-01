# tauri-plugin-automation

HTTP automation server for Tauri apps. Enables external control and automated testing.

**Rust crate:** Tauri v2 only
**JavaScript API:** Supports both Tauri v1 and v2

## Why?

Tauri's WebDriver support doesn't work on macOS (WKWebView lacks a driver). This plugin provides a lightweight HTTP API alternative that works everywhere.

## Features

- **HTTP API** on port 9876 for external control
- **DOM commands**: click, type, getText, navigate, waitFor, etc.
- **Screenshots**: Capture the current page as PNG
- **Works on macOS**: No WebDriver needed
- **Scriptable**: Perfect for automated testing tools and AI assistants

## Installation

### Rust (Cargo.toml)

```toml
[dependencies]
# From crates.io
tauri-plugin-automation-server = "0.2"

# Or from git
tauri-plugin-automation-server = { git = "https://github.com/dcherrera/tauri-plugin-automation" }
```

> **Note:** The Rust crate requires Tauri v2. For Tauri v1 projects, you can still use the JavaScript API
> which auto-detects the Tauri version, but the Rust HTTP server won't be available.

### Rust (main.rs)

```rust
use tauri_plugin_automation_server;

// Define your own command that calls the plugin
#[tauri::command]
fn automation_screenshot_data(data: String) {
    tauri_plugin_automation_server::set_screenshot_data(data);
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Start automation server
            tauri_plugin_automation_server::start_server(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            automation_screenshot_data,
            // ... your other commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### JavaScript (npm)

```bash
npm install tauri-plugin-automation-api
```

```typescript
import { initAutomation, initAutomationSync } from 'tauri-plugin-automation-api'

// Async version (recommended)
await initAutomation()

// Or sync version for boot files
initAutomationSync()
```

Alternatively, copy `guest-js/src/` to your project and import from there.

## API

### Health Check

```bash
curl http://localhost:9876/automation/health
```

```json
{"status": "ok", "port": 9876, "version": "0.2.0", "tauri": "v2"}
```

### Execute Command

```bash
curl -X POST http://localhost:9876/automation/execute \
  -H "Content-Type: application/json" \
  -d '{"command": "click", "args": {"selector": "#my-button"}}'
```

### Screenshot

```bash
curl http://localhost:9876/automation/screenshot --output screenshot.png
```

## Available Commands

| Command | Args | Description |
|---------|------|-------------|
| `navigate` | `{path}` | Navigate to route |
| `click` | `{selector}` | Click element |
| `type` | `{selector, text}` | Type into input |
| `clear` | `{selector}` | Clear input |
| `getText` | `{selector}` | Get element text |
| `getValue` | `{selector}` | Get input value |
| `getAttribute` | `{selector, attribute}` | Get attribute |
| `exists` | `{selector}` | Check if element exists |
| `waitFor` | `{selector, timeout?}` | Wait for element |
| `select` | `{selector, value}` | Select dropdown option |
| `check` | `{selector}` | Check checkbox |
| `uncheck` | `{selector}` | Uncheck checkbox |
| `getUrl` | - | Get current URL |
| `getTitle` | - | Get page title |
| `focus` | `{selector}` | Focus element |
| `blur` | `{selector}` | Blur element |
| `pressKey` | `{key, selector?}` | Press keyboard key |
| `scrollTo` | `{selector}` | Scroll to element |
| `submit` | `{selector}` | Submit form |
| `wait` | `{ms}` | Wait milliseconds |
| `eval` | `{script}` | Execute JavaScript |
| `getElements` | `{selector}` | Get all matching elements |
| `getHtml` | `{selector?}` | Get element HTML |

## Example: Automated Testing

```python
import requests

BASE = "http://localhost:9876/automation"

# Navigate to login
requests.post(f"{BASE}/execute", json={
    "command": "navigate",
    "args": {"path": "/login"}
})

# Fill form
requests.post(f"{BASE}/execute", json={
    "command": "type",
    "args": {"selector": "#email", "text": "test@example.com"}
})

requests.post(f"{BASE}/execute", json={
    "command": "type",
    "args": {"selector": "#password", "text": "password123"}
})

# Submit
requests.post(f"{BASE}/execute", json={
    "command": "click",
    "args": {"selector": "#login-button"}
})

# Take screenshot to verify
response = requests.get(f"{BASE}/screenshot")
with open("result.png", "wb") as f:
    f.write(response.content)
```

## Security

This plugin opens an HTTP server on localhost. For production:

1. **Disable in release builds** using Cargo features:

```toml
[features]
default = []  # Don't include automation by default
automation = ["tauri-plugin-automation-server"]
```

2. Only enable when needed:
```bash
cargo build --features automation
```

## Changelog

### v0.2.0
- **Breaking:** Rust crate now requires Tauri v2 (Cargo can't have both v1 and v2 due to native lib conflicts)
- JavaScript API auto-detects Tauri version (works with both v1 and v2)
- Added `initAutomationSync()` for boot files

### v0.1.0
- Initial release (Tauri v1 only)

## License

MIT Transparency License - see [LICENSE](LICENSE)
