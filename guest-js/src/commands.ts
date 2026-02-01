/**
 * Automation Commands
 *
 * DOM manipulation commands for automated testing.
 */

export type CommandArgs = Record<string, unknown>
export type CommandResult = unknown

type CommandHandler = (args: CommandArgs) => Promise<CommandResult>

// Router type for type safety
interface VueRouter {
  push: (path: string) => Promise<unknown>
  currentRoute: { value: { path: string } }
}

// Get Vue Router instance from the app
function getRouter(): VueRouter | undefined {
  // Access the router from the app instance stored on window
  const app = (window as unknown as { __VUE_APP__?: { config: { globalProperties: { $router: VueRouter } } } }).__VUE_APP__
  return app?.config.globalProperties.$router
}

/**
 * Wait for an element to appear in the DOM
 */
async function waitForElement(selector: string, timeout = 5000): Promise<Element | null> {
  const start = Date.now()

  while (Date.now() - start < timeout) {
    const element = document.querySelector(selector)
    if (element) return element
    await new Promise(r => setTimeout(r, 100))
  }

  return null
}

/**
 * Get an element or throw
 */
function getElement(selector: string): Element {
  const element = document.querySelector(selector)
  if (!element) {
    throw new Error(`Element not found: ${selector}`)
  }
  return element
}

/**
 * Simulate realistic typing into an input
 */
function simulateTyping(element: HTMLInputElement | HTMLTextAreaElement, text: string): void {
  element.focus()
  element.value = text

  // Dispatch events to trigger Vue reactivity
  element.dispatchEvent(new Event('input', { bubbles: true }))
  element.dispatchEvent(new Event('change', { bubbles: true }))
}

/**
 * Available automation commands
 */
export const commands: Record<string, CommandHandler> = {
  /**
   * Navigate to a route
   */
  async navigate(args): Promise<void> {
    const path = args.path as string
    if (!path) throw new Error('Missing path argument')

    const router = getRouter()
    if (router) {
      await router.push(path)
    } else {
      // Fallback: use window.location
      window.location.hash = path
    }

    // Wait for navigation to complete
    await new Promise(r => setTimeout(r, 100))
  },

  /**
   * Click an element
   */
  async click(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = await waitForElement(selector, 5000)
    if (!element) throw new Error(`Element not found: ${selector}`)

    // Scroll into view
    element.scrollIntoView({ behavior: 'instant', block: 'center' })
    await new Promise(r => setTimeout(r, 50))

    // Click
    ;(element as HTMLElement).click()

    // Wait for any resulting actions
    await new Promise(r => setTimeout(r, 100))
  },

  /**
   * Type text into an input
   */
  async type(args): Promise<void> {
    const selector = args.selector as string
    const text = args.text as string
    if (!selector) throw new Error('Missing selector argument')
    if (text === undefined) throw new Error('Missing text argument')

    const element = await waitForElement(selector, 5000)
    if (!element) throw new Error(`Element not found: ${selector}`)

    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
      simulateTyping(element, text)
    } else {
      throw new Error(`Element is not an input: ${selector}`)
    }
  },

  /**
   * Clear an input
   */
  async clear(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector)

    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
      simulateTyping(element, '')
    } else {
      throw new Error(`Element is not an input: ${selector}`)
    }
  },

  /**
   * Get element text content
   */
  async getText(args): Promise<string> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = await waitForElement(selector, 5000)
    if (!element) throw new Error(`Element not found: ${selector}`)

    return element.textContent?.trim() || ''
  },

  /**
   * Get input value
   */
  async getValue(args): Promise<string> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector)

    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element instanceof HTMLSelectElement) {
      return element.value
    }

    throw new Error(`Element is not an input: ${selector}`)
  },

  /**
   * Get element attribute
   */
  async getAttribute(args): Promise<string | null> {
    const selector = args.selector as string
    const attribute = args.attribute as string
    if (!selector) throw new Error('Missing selector argument')
    if (!attribute) throw new Error('Missing attribute argument')

    const element = getElement(selector)
    return element.getAttribute(attribute)
  },

  /**
   * Check if element exists
   */
  async exists(args): Promise<boolean> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    return document.querySelector(selector) !== null
  },

  /**
   * Wait for element to appear
   */
  async waitFor(args): Promise<boolean> {
    const selector = args.selector as string
    const timeout = (args.timeout as number) || 5000
    if (!selector) throw new Error('Missing selector argument')

    const element = await waitForElement(selector, timeout)
    return element !== null
  },

  /**
   * Select dropdown option
   */
  async select(args): Promise<void> {
    const selector = args.selector as string
    const value = args.value as string
    if (!selector) throw new Error('Missing selector argument')
    if (value === undefined) throw new Error('Missing value argument')

    const element = getElement(selector)

    if (element instanceof HTMLSelectElement) {
      element.value = value
      element.dispatchEvent(new Event('change', { bubbles: true }))
    } else {
      throw new Error(`Element is not a select: ${selector}`)
    }
  },

  /**
   * Check a checkbox
   */
  async check(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector)

    if (element instanceof HTMLInputElement && element.type === 'checkbox') {
      if (!element.checked) {
        element.click()
      }
    } else {
      throw new Error(`Element is not a checkbox: ${selector}`)
    }
  },

  /**
   * Uncheck a checkbox
   */
  async uncheck(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector)

    if (element instanceof HTMLInputElement && element.type === 'checkbox') {
      if (element.checked) {
        element.click()
      }
    } else {
      throw new Error(`Element is not a checkbox: ${selector}`)
    }
  },

  /**
   * Get current URL/route
   */
  async getUrl(): Promise<string> {
    return window.location.href
  },

  /**
   * Get page title
   */
  async getTitle(): Promise<string> {
    return document.title
  },

  /**
   * Evaluate arbitrary JavaScript
   */
  async eval(args): Promise<unknown> {
    const script = args.script as string
    if (!script) throw new Error('Missing script argument')

    // Use Function constructor to evaluate in global scope
    const fn = new Function(`return (async () => { ${script} })()`)
    return await fn()
  },

  /**
   * Get all elements matching selector
   */
  async getElements(args): Promise<{ count: number; texts: string[] }> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const elements = document.querySelectorAll(selector)
    const texts = Array.from(elements).map(el => el.textContent?.trim() || '')

    return { count: elements.length, texts }
  },

  /**
   * Scroll to element
   */
  async scrollTo(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector)
    element.scrollIntoView({ behavior: 'smooth', block: 'center' })

    await new Promise(r => setTimeout(r, 300))
  },

  /**
   * Get page HTML
   */
  async getHtml(args): Promise<string> {
    const selector = (args.selector as string) || 'body'
    const element = getElement(selector)
    return element.innerHTML
  },

  /**
   * Wait for a specified time
   */
  async wait(args): Promise<void> {
    const ms = (args.ms as number) || 1000
    await new Promise(r => setTimeout(r, ms))
  },

  /**
   * Focus an element
   */
  async focus(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector) as HTMLElement
    element.focus()
  },

  /**
   * Blur (unfocus) an element
   */
  async blur(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector) as HTMLElement
    element.blur()
  },

  /**
   * Press a key
   */
  async pressKey(args): Promise<void> {
    const key = args.key as string
    const selector = args.selector as string
    if (!key) throw new Error('Missing key argument')

    const target = selector ? getElement(selector) : document.activeElement || document.body

    target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }))
    target.dispatchEvent(new KeyboardEvent('keyup', { key, bubbles: true }))
  },

  /**
   * Submit a form
   */
  async submit(args): Promise<void> {
    const selector = args.selector as string
    if (!selector) throw new Error('Missing selector argument')

    const element = getElement(selector)

    if (element instanceof HTMLFormElement) {
      element.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    } else {
      // Find parent form
      const form = element.closest('form')
      if (form) {
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
      } else {
        throw new Error('No form found')
      }
    }
  },
}
