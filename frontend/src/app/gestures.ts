import { useEffect } from 'react'

import { setUnderlay, useInUnderlay } from '../shared/underlay'
import { telegramSupporting } from './telegram'

/**
 * Keeps a swipe down from closing the app in the middle of a workout. Scrolling
 * still works, and so does Telegram's own close button.
 */
export function useNoSwipeToClose() {
  const underlay = useInUnderlay()
  useEffect(() => {
    const webApp = telegramSupporting('7.7')
    if (!webApp || underlay) return
    webApp.disableVerticalSwipes()
    return () => webApp.enableVerticalSwipes()
  }, [underlay])
}

/** A swipe has to start this close to the left edge. */
const EDGE_PX = 24
/** Movement before a touch counts as a swipe or a scroll. */
const SLOP_PX = 10
/** Released past this share of the screen, the swipe goes back… */
const BACK_SHARE = 0.33
/** …or at this speed (px per ms), however short. Moving back left this fast cancels it. */
const FLICK_SPEED = 0.4
const SETTLE_MS = 180

/** What a swipe from the left edge does on this page: the same as the back arrow. */
let swipeBack: { back: () => void; destination: () => string } | null = null

/**
 * Makes `back` the swipe's action until the returned function is called;
 * `destination` is the address it goes to, drawn under the page meanwhile.
 */
export function setSwipeBack(back: () => void, destination: () => string): () => void {
  const action = { back, destination }
  swipeBack = action
  return () => {
    if (swipeBack === action) swipeBack = null
  }
}

/** Calls `then` in the first frame after `left` is no longer the screen shown, or in half a second. */
function whenScreenLeft(left: string | undefined, then: () => void) {
  const html = document.documentElement
  const until = performance.now() + 500
  const check = () => {
    if (html.dataset.screen !== left || performance.now() > until) then()
    else requestAnimationFrame(check)
  }
  requestAnimationFrame(check)
}

/** Kept once the phone has shown it swipes back by itself all the same. */
const SYSTEM_SWIPE_KEY = 'system_swipe_back'

function systemSwipesBack(): boolean {
  try {
    return localStorage.getItem(SYSTEM_SWIPE_KEY) === '1'
  } catch {
    return false
  }
}

/**
 * Going back with a swipe from the left edge, as iPhone apps do: the page
 * follows the finger over the screen it goes back to and goes back past a
 * third of the screen or on a quick flick, otherwise it springs back.
 * Telegram gives Mini Apps no such gesture on iPhones; newer iPhones give apps
 * on the home screen one, which this turns off while it is on. On Android the
 * system one already goes back. Returns a function that removes it.
 */
export function installSwipeBack(): () => void {
  const root = document.getElementById('root')
  if (!root || systemSwipesBack()) return () => undefined
  const html = document.documentElement
  let start: { x: number; y: number } | null = null
  let swiping = false
  let offset = 0
  let last = { x: 0, time: 0 }
  let speed = 0
  // Set when the phone went back with its own swipe anyway: two swipes show
  // two screens behind the page and go back twice, so the app's steps aside.
  let systemSwiped = false
  // Asks the phone to leave the swipe to the app.
  html.style.overscrollBehaviorX = 'none'
  document.body.style.overscrollBehaviorX = 'none'

  // `left`, not a transform, so fixed parts (the tab bar, the set editor) stay put.
  const place = (x: number, animate: boolean) => {
    root.style.transition = animate ? `left ${SETTLE_MS}ms ease-out` : 'none'
    root.style.left = `${x}px`
    // How far the screen underneath has come out (`.swipe-underlay`).
    html.classList.toggle('swipe-settling', animate)
    html.style.setProperty('--swipe-progress', String(x / window.innerWidth))
  }
  const reset = () => {
    html.classList.remove('swiping-back', 'swipe-settling')
    html.style.removeProperty('--swipe-progress')
    root.style.transition = ''
    root.style.left = ''
    setUnderlay(null)
  }

  const onStart = (event: TouchEvent) => {
    start = null
    const touch = event.touches.item(0)
    if (systemSwiped || !swipeBack || !touch || event.touches.length > 1 || touch.clientX > EDGE_PX) return
    // A sheet or dialog on top has its own way out.
    if (document.querySelector('[aria-modal="true"]')) return
    start = { x: touch.clientX, y: touch.clientY }
    last = { x: touch.clientX, time: event.timeStamp }
    offset = 0
    speed = 0
  }

  const onMove = (event: TouchEvent) => {
    const touch = event.touches.item(0)
    if (!start || !touch) return
    const dx = touch.clientX - start.x
    const dy = touch.clientY - start.y
    if (!swiping) {
      // Up or down first: a scroll, not a swipe.
      if (event.touches.length > 1 || (Math.abs(dy) > SLOP_PX && Math.abs(dy) >= dx)) {
        start = null
        return
      }
      if (dx < SLOP_PX) return
      swiping = true
      html.classList.add('swiping-back')
      setUnderlay(swipeBack?.destination() ?? null)
    }
    event.preventDefault()
    const elapsed = event.timeStamp - last.time
    if (elapsed > 0) speed = (touch.clientX - last.x) / elapsed
    last = { x: touch.clientX, time: event.timeStamp }
    offset = Math.max(0, dx)
    place(offset, false)
  }

  const onEnd = (event: TouchEvent) => {
    if (!start) return
    start = null
    if (!swiping) return
    swiping = false
    const back = swipeBack
    const width = window.innerWidth
    const far = offset > width * BACK_SHARE
    const flick = speed > FLICK_SPEED && offset > 3 * SLOP_PX
    const changedMind = speed < -FLICK_SPEED
    const goBack = back !== null && event.type === 'touchend' && !changedMind && (far || flick)
    place(goBack ? width : 0, true)
    window.setTimeout(() => {
      if (systemSwiped) return remove()
      if (!goBack || back === null) return reset()
      const left = html.dataset.screen
      back.back()
      // The page comes back to its place once the next screen is in it, so
      // the old one does not flash back.
      whenScreenLeft(left, reset)
    }, SETTLE_MS)
  }

  const onPop = (event: PopStateEvent) => {
    if (!event.hasUAVisualTransition) return
    systemSwiped = true
    try {
      localStorage.setItem(SYSTEM_SWIPE_KEY, '1')
    } catch {
      // Found out again next time.
    }
    // Mid-swipe, the swipe's own timer finishes and removes it.
    if (!swiping && !html.classList.contains('swipe-settling')) remove()
  }

  document.addEventListener('touchstart', onStart, { passive: true })
  // Not passive: a swipe stops the page from scrolling under it.
  document.addEventListener('touchmove', onMove, { passive: false })
  document.addEventListener('touchend', onEnd)
  document.addEventListener('touchcancel', onEnd)
  window.addEventListener('popstate', onPop)
  function remove() {
    document.removeEventListener('touchstart', onStart)
    document.removeEventListener('touchmove', onMove)
    document.removeEventListener('touchend', onEnd)
    document.removeEventListener('touchcancel', onEnd)
    window.removeEventListener('popstate', onPop)
    html.style.overscrollBehaviorX = ''
    document.body.style.overscrollBehaviorX = ''
    reset()
  }
  return remove
}
