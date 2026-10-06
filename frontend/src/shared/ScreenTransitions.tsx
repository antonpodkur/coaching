import { useEffect, useLayoutEffect, useRef } from 'react'
import { useLocation, useNavigationType } from 'react-router'

import { screenMotion, scrolled } from './screenMotion'

/**
 * Sets up each screen change as it happens (see screenMotion.ts): which way
 * the transition runs, and where the new screen is scrolled, both before the
 * browser takes its picture of it. As on an iPhone, a new screen opens at the
 * top and going back finds a screen where it was left.
 */
export function ScreenTransitions() {
  const { pathname } = useLocation()
  const historyStep = useNavigationType() === 'POP'
  const shown = useRef(pathname)

  useLayoutEffect(() => {
    const from = shown.current
    if (from === pathname) return
    shown.current = pathname
    // For the swipe back (app/gestures.ts) to see the screen it went to is in.
    document.documentElement.dataset.screen = pathname
    const motion = screenMotion(from, pathname, historyStep)
    document.documentElement.dataset.screenMotion = motion
    const back = motion === 'back' || motion === 'fade'
    window.scrollTo(0, back ? (scrolled.get(pathname) ?? 0) : 0)
  }, [pathname, historyStep])

  // Also kept while scrolling, for steps back through history (Android's back
  // gesture), which do not pass through the app's navigation.
  useEffect(() => {
    const keep = () => scrolled.set(shown.current, window.scrollY)
    window.addEventListener('scroll', keep, { passive: true })
    return () => window.removeEventListener('scroll', keep)
  }, [])

  return null
}
