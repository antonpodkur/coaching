import { useLocation } from 'react-router'

/** Where a page's back arrow leads, and what it says. */
export interface BackTarget {
  to: string
  label: string
}

/**
 * The page that opened this one, if it said so in the link's state (the
 * workouts tab does, so back returns to its week and filters), else `fallback`.
 */
export function useBackTarget(fallback: BackTarget): BackTarget {
  const state: unknown = useLocation().state
  if (state && typeof state === 'object' && 'back' in state) {
    const back = (state as { back: BackTarget }).back
    if (typeof back.to === 'string' && typeof back.label === 'string') return back
  }
  return fallback
}
