import type { createBrowserRouter } from 'react-router'

/**
 * How one screen gives way to the next, as on an iPhone: a deeper screen
 * slides in from the right over the one before, going back slides it off to
 * the right again, and Dasha's tabs fade into each other. The browser's view
 * transitions play it from pictures of the two screens, so scrolling and the
 * fixed bars need no care; phones without them just switch.
 *
 * "Back" is what the back arrows (and the swipe) do, a step back through
 * history, or any move to a shallower screen. The back arrows open the page
 * above rather than stepping through history, so they say so (`goingBack`).
 */
export type ScreenMotion = 'forward' | 'back' | 'fade' | 'none'

type Router = ReturnType<typeof createBrowserRouter>

/** Dasha's sections behind the tab bar. */
const TABS = new Set(['/app', '/app/workouts', '/app/exercises'])

/** How the last navigation was asked for: replacing its screen (a new exercise's form with its page), or going back. */
let intent = { replace: false, back: false }
/** Set by a back arrow just before it navigates. */
let backRequested = false

/** Where each screen was scrolled to when left, for coming back to it. */
export const scrolled = new Map<string, number>()

function depth(pathname: string) {
  return pathname.split('/').filter(Boolean).length
}

/** The next navigation goes back, e.g. from the builder to its client's page, deep as both are. */
export function goingBack() {
  backRequested = true
}

/** `historyStep`: a step through history, e.g. Android's back gesture, not a new screen. */
export function screenMotion(from: string, to: string, historyStep = false): ScreenMotion {
  if (from === to) return 'none'
  if (TABS.has(from) && TABS.has(to)) return 'fade'
  if (historyStep || intent.back) return 'back'
  const deeper = depth(to) - depth(from)
  if (deeper < 0) return 'back'
  return deeper === 0 && intent.replace ? 'fade' : 'forward'
}

/**
 * Every navigation, from links, back arrows and notifications alike, notes
 * where the screen it leaves was scrolled, and plays a screen transition
 * where the phone can. Not when the screen stays (a filter in its address, a
 * tap on the tab already open), after a swipe back (the page already slid
 * away under the finger), or after a step back the browser animated itself
 * (Safari's swipe from the edge): playing one then shows the old screen again
 * and slides it away a second time.
 */
export function playScreenTransitions(router: Router) {
  const navigate = router.navigate.bind(router)
  const subscribe = router.subscribe.bind(router)
  const canPlay = typeof document.startViewTransition === 'function'
  const lessMotion = window.matchMedia('(prefers-reduced-motion: reduce)')
  let browserAnimated = false

  // Before React Router's own listener, which starts the step.
  window.addEventListener(
    'popstate',
    (event) => {
      // Not known to older phones, which then animate nothing themselves.
      browserAnimated = event.hasUAVisualTransition === true
    },
    { capture: true },
  )

  router.navigate = ((to: Parameters<Router['navigate']>[0], options?: Parameters<Router['navigate']>[1]) => {
    // A step through history, e.g. a back arrow to the screen before: React
    // Router plays it if the step forward to there played one.
    if (typeof to === 'number') {
      backRequested = false
      return navigate(to)
    }
    browserAnimated = false
    scrolled.set(window.location.pathname, window.scrollY)
    intent = { replace: options?.replace === true, back: backRequested }
    backRequested = false
    return navigate(to, { ...options, viewTransition: options?.viewTransition ?? canPlay })
  }) as Router['navigate']

  // Whether it plays is decided here, once the router knows both screens.
  router.subscribe = (subscriber) =>
    subscribe((state, opts) => {
      const transition = opts.viewTransitionOpts
      const plays =
        transition !== undefined &&
        transition.currentLocation.pathname !== transition.nextLocation.pathname &&
        !browserAnimated &&
        !lessMotion.matches &&
        !document.documentElement.classList.contains('swiping-back')
      subscriber(state, plays ? opts : { ...opts, viewTransitionOpts: undefined })
    })
}
