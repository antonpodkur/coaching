import { demo } from './mode'

/**
 * Imported first by main.tsx. In the demo, the app's storage is swapped for
 * memory before any other module touches it: the demo shares the real app's
 * address, so it must never read a visitor's real session or queue its
 * made-up sets into the real app's outbox.
 */

class MemoryStorage {
  #items = new Map<string, string>()

  get length() {
    return this.#items.size
  }

  key(index: number) {
    return [...this.#items.keys()][index] ?? null
  }

  getItem(key: string) {
    return this.#items.get(key) ?? null
  }

  setItem(key: string, value: string) {
    this.#items.set(key, String(value))
  }

  removeItem(key: string) {
    this.#items.delete(key)
  }

  clear() {
    this.#items.clear()
  }
}

if (demo) {
  for (const name of ['localStorage', 'sessionStorage'] as const) {
    Object.defineProperty(window, name, { value: new MemoryStorage(), configurable: true })
  }
  // A phone shows no scrollbars (styles.css).
  document.documentElement.dataset.demo = demo.role
}
