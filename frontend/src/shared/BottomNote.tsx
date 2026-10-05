import type { ReactNode } from 'react'
import { createPortal } from 'react-dom'

/**
 * A note floating over the bottom of the screen, such as "no signal" or "new
 * version". It covers the page instead of pushing its buttons around. All
 * notes go into one stack (`#notes` in index.html), so two never overlap.
 */
export function BottomNote({ className, children }: { className: string; children: ReactNode }) {
  const note = (
    <div className={className} role="status">
      {children}
    </div>
  )
  const stack = document.getElementById('notes')
  return stack ? createPortal(note, stack) : note
}
