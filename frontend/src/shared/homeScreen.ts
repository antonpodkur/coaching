import { useSyncExternalStore } from 'react'

import { type HomeScreenStatus, telegramSupporting } from '../app/telegram'

let checked = false
let status: HomeScreenStatus | null = null
const listeners = new Set<() => void>()

function setStatus(next: HomeScreenStatus) {
  status = next
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** `null` until Telegram has answered, or where it cannot tell. */
export function useHomeScreenStatus(): HomeScreenStatus | null {
  return useSyncExternalStore(subscribe, () => status)
}

/**
 * Asks Telegram whether the icon is on the home screen. Called once as the app
 * starts, so the answer is in before the first page draws.
 */
export function checkHomeScreen() {
  const webApp = telegramSupporting('8.0')
  if (!webApp || checked) return
  checked = true
  webApp.checkHomeScreenStatus(setStatus)
  webApp.onEvent('homeScreenAdded', () => setStatus('added'))
}
