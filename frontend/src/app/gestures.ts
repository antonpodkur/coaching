import { useEffect } from 'react'

import { telegramSupporting } from './telegram'

/**
 * Keeps a swipe down from closing the app in the middle of a workout. Scrolling
 * still works, and so does Telegram's own close button.
 */
export function useNoSwipeToClose() {
  useEffect(() => {
    const webApp = telegramSupporting('7.7')
    if (!webApp) return
    webApp.disableVerticalSwipes()
    return () => webApp.enableVerticalSwipes()
  }, [])
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
let swipeBack: (() => void) | null = null

/** Makes `back` the swipe's action until the returned function is called. */
export function setSwipeBack(back: () => void): () => void {
  swipeBack = back
  return () => {
    if (swipeBack === back) swipeBack = null
  }
}

/**
 * Going back with a swipe from the left edge, as iPhone apps do: the page
 * follows the finger and goes back past a third of the screen or on a quick
 * flick, otherwise it springs back. Telegram gives Mini Apps no such gesture
 * on iPhones; on Android the system one already presses the back arrow.
 * Returns a function that removes it.
 */
export function installSwipeBack(): () => void {
  const root = document.getElementById('root')
  if (!root) return () => undefined
  const html = document.documentElement
  let start: { x: number; y: number } | null = null
  let swiping = false
  let offset = 0
  let last = { x: 0, time: 0 }
  let speed = 0

  // `left`, not a transform, so fixed parts (the tab bar, the set editor) stay put.
  const place = (x: number, animate: boolean) => {
    root.style.transition = animate ? `left ${SETTLE_MS}ms ease-out` : 'none'
    root.style.left = `${x}px`
  }
  const reset = () => {
    html.classList.remove('swiping-back')
    root.style.transition = ''
    root.style.left = ''
  }

  const onStart = (event: TouchEvent) => {
    start = null
    const touch = event.touches.item(0)
    if (!swipeBack || !touch || event.touches.length > 1 || touch.clientX > EDGE_PX) return
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
      if (goBack) back()
      // Wait for the next page to draw, so the old one does not flash back.
      requestAnimationFrame(() => requestAnimationFrame(reset))
    }, SETTLE_MS)
  }

  document.addEventListener('touchstart', onStart, { passive: true })
  // Not passive: a swipe stops the page from scrolling under it.
  document.addEventListener('touchmove', onMove, { passive: false })
  document.addEventListener('touchend', onEnd)
  document.addEventListener('touchcancel', onEnd)
  return () => {
    document.removeEventListener('touchstart', onStart)
    document.removeEventListener('touchmove', onMove)
    document.removeEventListener('touchend', onEnd)
    document.removeEventListener('touchcancel', onEnd)
    reset()
  }
}
