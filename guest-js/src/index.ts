/**
 * Tauri Automation Service
 *
 * Provides automation capabilities for external testing tools.
 * Only active in development builds.
 */

import { invoke } from '@tauri-apps/api/tauri'
import { commands, type CommandArgs, type CommandResult } from './commands'

declare global {
  interface Window {
    __TAURI_AUTOMATION__: AutomationService
  }
}

export interface AutomationService {
  execute: (command: string, args: CommandArgs) => Promise<CommandResult>
  screenshot: () => Promise<string>
  captureAndSend: () => Promise<void>
  _lastResult: { success: boolean; result?: unknown; error?: string } | null
}

/**
 * Initialize the automation service
 * Call this once when your app starts
 * Note: Automation is controlled by Rust feature flag, JS side always initializes
 */
export function initAutomation(): void {
  console.log('[Automation] Initializing...')

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
      // Dynamically import html2canvas
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
