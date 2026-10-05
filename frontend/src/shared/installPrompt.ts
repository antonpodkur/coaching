import { useSyncExternalStore } from 'react'

/** Chrome's own install dialog, offered through `beforeinstallprompt` (Android, computers). */
export interface InstallPrompt extends Event {
  prompt(): Promise<void>
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>
}

let offered: InstallPrompt | null = null
const listeners = new Set<() => void>()

function setOffered(next: InstallPrompt | null) {
  offered = next
  for (const listener of listeners) listener()
}

/**
 * Keeps Chrome's install offer for the install page's "Встановити". Listens
 * from the start, since Chrome may offer it before that page draws. Elsewhere
 * Chrome shows its own small banner as usual.
 */
export function listenForInstallPrompt() {
  window.addEventListener('beforeinstallprompt', (event) => {
    if (window.location.pathname.startsWith('/install')) event.preventDefault()
    setOffered(event as InstallPrompt)
  })
  window.addEventListener('appinstalled', () => setOffered(null))
}

/** Chrome's install offer, or `null` where there is none (iPhones, or already installed). */
export function useInstallPrompt(): InstallPrompt | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
    () => offered,
  )
}
