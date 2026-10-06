import { createContext, use, useSyncExternalStore } from 'react'

/**
 * The screen a swipe back goes to, drawn under the page while the finger pulls
 * it away (app/gestures.ts, shared/SwipeUnderlay.tsx), as on an iPhone.
 */
let underlay: string | null = null
const listeners = new Set<() => void>()

export function setUnderlay(location: string | null) {
  if (location === underlay) return
  underlay = location
  for (const listener of listeners) listener()
}

export function useUnderlay(): string | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    () => underlay,
  )
}

/**
 * Inside that copy of the screen. It is only a picture, so it leaves Telegram's
 * back arrow, the swipe and the like to the screen on top.
 */
export const InUnderlay = createContext(false)

export function useInUnderlay(): boolean {
  return use(InUnderlay)
}
