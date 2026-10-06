import type { NavigateFunction, createBrowserRouter } from 'react-router'

import { goingBack } from './screenMotion'

type Router = ReturnType<typeof createBrowserRouter>

/**
 * Where each step of this tab's history leads, by React Router's number for
 * it (`history.state.idx`), so going back can step back through history to a
 * screen already in it instead of adding a step. Kept for a reload.
 */
const STORAGE_KEY = 'history_steps'
const steps: string[] = load()

function load(): string[] {
  try {
    const saved: unknown = JSON.parse(sessionStorage.getItem(STORAGE_KEY) ?? '[]')
    return Array.isArray(saved) ? saved.map(String) : []
  } catch {
    return []
  }
}

function step(): number | null {
  const idx: unknown = (window.history.state as { idx?: unknown } | null)?.idx
  return typeof idx === 'number' ? idx : null
}

/** Notes every screen the app shows at the step it is shown at. */
export function trackHistory(router: Router) {
  const note = (location: { pathname: string; search: string }, pushed: boolean) => {
    const at = step()
    if (at === null) return
    // A new step drops the ones it was ahead of.
    if (pushed) steps.length = Math.min(steps.length, at)
    steps[at] = location.pathname + location.search
    try {
      sessionStorage.setItem(STORAGE_KEY, JSON.stringify(steps))
    } catch {
      // Private browsing: going back then adds steps, as it did before.
    }
  }
  note(router.state.location, false)
  router.subscribe((state) => note(state.location, state.historyAction === 'PUSH'))
}

/** How many steps back the nearest earlier step showing `to`'s screen is, if any. */
function stepsBackTo(to: string): number | null {
  const at = step()
  if (at === null) return null
  const [pathname] = to.split('?')
  for (let earlier = at - 1; earlier >= 0; earlier -= 1) {
    const shown = steps[earlier]
    if (shown === undefined) return null
    // A back arrow names the screen; the step keeps its filters, e.g. a week.
    if (to.includes('?') ? shown === to : shown.split('?')[0] === pathname) return at - earlier
  }
  return null
}

/** The address going back to `to` lands on: the earlier step's, filters and all, else `to`. */
export function backDestination(to: string): string {
  const back = stepsBackTo(to)
  const at = step()
  return back === null || at === null ? to : (steps[at - back] ?? to)
}

/**
 * Goes back to `to`, as a native app does: back through history if the
 * screen is in it, so the system back button (Android outside Telegram) does
 * not lead to the screen just left; else in place of this screen.
 */
export function goBackTo(navigate: NavigateFunction, to: string) {
  goingBack()
  const back = stepsBackTo(to)
  void (back === null ? navigate(to, { replace: true }) : navigate(-back))
}
