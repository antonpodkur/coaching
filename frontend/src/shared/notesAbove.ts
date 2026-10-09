import { type RefObject, useEffect } from 'react'

/** Bars fixed to the bottom of the screen, by height (`useNotesAbove`). */
const bars = new Map<Element, number>()

function placeNotes() {
  const lift = Math.max(0, ...bars.values())
  const root = document.documentElement
  root.style.setProperty('--notes-lift', `${lift}px`)
  root.classList.toggle('notes-lifted', lift > 0)
}

/**
 * Keeps the notes above a bar fixed to the bottom of the screen, such as the
 * builder's, instead of over its buttons. Follows the bar's height as it changes.
 */
export function useNotesAbove(bar: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const node = bar.current
    if (!node) return
    const observer = new ResizeObserver(() => {
      bars.set(node, node.offsetHeight)
      placeNotes()
    })
    observer.observe(node)
    return () => {
      observer.disconnect()
      bars.delete(node)
      placeNotes()
    }
  }, [bar])
}
