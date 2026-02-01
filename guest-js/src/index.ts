/**
 * Tauri Automation Service
 *
 * Provides automation capabilities for external testing tools.
 * Supports both Tauri v1 (@tauri-apps/api/tauri) and v2 (@tauri-apps/api/core).
 */

import { commands, type CommandArgs, type CommandResult } from './commands'

declare global {
  interface Window {
    __TAURI_AUTOMATION__: AutomationService
    __TAURI__?: unknown
    __TAURI_INTERNALS__?: unknown
  }
}

export interface AutomationService {
  execute: (command: string, args: CommandArgs) => Promise<CommandResult>
  screenshot: () => Promise<string>
  captureAndSend: () => Promise<void>
  _lastResult: { success: boolean; result?: unknown; error?: string } | null
}

/**
 * Detect Tauri version and get the invoke function
 * Supports both Tauri v1 and v2
 */
async function getInvoke(): Promise<(cmd: string, args?: Record<string, unknown>) => Promise<unknown>> {
  // Tauri v2: uses @tauri-apps/api/core
  try {
    // Dynamic import with variable to avoid TypeScript module resolution
    const v2Module = '@tauri-apps/api/core'
    const core = await import(/* webpackIgnore: true */ v2Module) as { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> }
    if (core.invoke) {
      console.log('[Automation] Using Tauri v2 API')
      return core.invoke
    }
  } catch {
    // Not Tauri v2
  }

  // Tauri v1: uses @tauri-apps/api/tauri
  try {
    // Dynamic import with variable to avoid TypeScript module resolution
    const v1Module = '@tauri-apps/api/tauri'
    const tauri = await import(/* webpackIgnore: true */ v1Module) as { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> }
    if (tauri.invoke) {
      console.log('[Automation] Using Tauri v1 API')
      return tauri.invoke
    }
  } catch {
    // Not Tauri v1
  }

  // Fallback: try window.__TAURI__ (works with both versions)
  if (window.__TAURI__) {
    const t = window.__TAURI__ as { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> }
    if (t.invoke) {
      console.log('[Automation] Using window.__TAURI__ fallback')
      return t.invoke
    }
  }

  throw new Error('Tauri invoke not available - ensure @tauri-apps/api v1 or v2 is installed')
}

/**
 * Initialize the automation service
 * Call this once when your app starts
 */
export async function initAutomation(): Promise<void> {
  console.log('[Automation] Initializing...')

  let invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>
  try {
    invoke = await getInvoke()
  } catch (e) {
    console.warn('[Automation] Tauri invoke not available, some features may not work:', e)
    invoke = async () => { throw new Error('Tauri not available') }
  }

  const service: AutomationService = {
    _lastResult: null,

    /**
     * Execute an automation command
     */
    async execute(command: string, args: CommandArgs = {}): Promise<CommandResult> {
      console.log(`[Automation] Executing: ${command}`, args)

      const handler = commands[command]
      if (!handler) {
        throw new Error(`Unknown command: ${command}`)
      }

      try {
        const result = await handler(args)
        console.log(`[Automation] Result:`, result)
        return result
      } catch (error) {
        console.error(`[Automation] Error:`, error)
        throw error
      }
    },

    /**
     * Capture screenshot and return as data URL
     */
    async screenshot(): Promise<string> {
      const html2canvas = await loadHtml2Canvas()
      const canvas = await html2canvas(document.body, {
        backgroundColor: '#121212',
        scale: 1,
        logging: false,
        useCORS: true,
      })
      return canvas.toDataURL('image/png')
    },

    /**
     * Capture screenshot and send to Rust backend
     */
    async captureAndSend(): Promise<void> {
      try {
        const dataUrl = await this.screenshot()
        await invoke('automation_screenshot_data', { data: dataUrl })
        console.log('[Automation] Screenshot sent to backend')
      } catch (error) {
        console.error('[Automation] Screenshot capture failed:', error)
        throw error
      }
    },
  }

  // Expose globally for Rust to call via eval
  window.__TAURI_AUTOMATION__ = service

  console.log('[Automation] Ready. HTTP API available at http://localhost:9876')
}

// Synchronous version for boot files that can't use async
export function initAutomationSync(): void {
  initAutomation().catch(e => {
    console.error('[Automation] Failed to initialize:', e)
  })
}

// html2canvas function type
type Html2CanvasFn = (element: HTMLElement, options?: Record<string, unknown>) => Promise<HTMLCanvasElement>

/**
 * Dynamically load html2canvas from CDN
 */
async function loadHtml2Canvas(): Promise<Html2CanvasFn> {
  if ((window as unknown as { html2canvas?: Html2CanvasFn }).html2canvas) {
    return (window as unknown as { html2canvas: Html2CanvasFn }).html2canvas
  }

  return new Promise((resolve, reject) => {
    const script = document.createElement('script')
    script.src = 'https://cdnjs.cloudflare.com/ajax/libs/html2canvas/1.4.1/html2canvas.min.js'
    script.onload = () => {
      const h2c = (window as unknown as { html2canvas: Html2CanvasFn }).html2canvas
      if (h2c) {
        resolve(h2c)
      } else {
        reject(new Error('html2canvas failed to load'))
      }
    }
    script.onerror = () => reject(new Error('Failed to load html2canvas'))
    document.head.appendChild(script)
  })
}

export { commands } from './commands'
export type { CommandArgs, CommandResult } from './commands'
