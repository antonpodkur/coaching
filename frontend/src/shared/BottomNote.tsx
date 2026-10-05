import { AnimatePresence, m } from 'motion/react'
import type { ReactNode } from 'react'
import { createPortal } from 'react-dom'

/**
 * A note floating over the bottom of the screen, such as "no signal" or "new
 * version". It covers the page instead of pushing its buttons around, and
 * slides up into view and back down. All notes go into one stack (`#notes`
 * in index.html), so two never overlap.
 */
export function BottomNote({
  show,
  className,
  children,
}: {
  show: boolean
  className: string
  children: ReactNode
}) {
  const note = (
    <AnimatePresence>
      {show && (
        <m.div
          className={className}
          role="status"
          layout="position"
          initial={{ opacity: 0, y: 24, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 24, scale: 0.96 }}
        >
          {children}
        </m.div>
      )}
    </AnimatePresence>
  )
  const stack = document.getElementById('notes')
  return stack ? createPortal(note, stack) : note
}
