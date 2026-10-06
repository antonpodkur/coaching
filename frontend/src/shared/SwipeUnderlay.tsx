import type { ReactNode } from 'react'

import { scrolled } from './screenMotion'
import { InUnderlay, useUnderlay } from './underlay'

/**
 * The screen a swipe back goes to, under the page the finger pulls away and
 * scrolled where it was left, coming out of the dark as in the back slide.
 * `render` draws the app's screens at that address.
 */
export function SwipeUnderlay({ render }: { render: (location: string) => ReactNode }) {
  const location = useUnderlay()
  if (location === null) return null
  const pathname = location.replace(/[?#].*/, '')
  return (
    <InUnderlay value>
      <div className="swipe-underlay" aria-hidden="true" inert>
        <div style={{ marginTop: -(scrolled.get(pathname) ?? 0) }}>{render(location)}</div>
      </div>
    </InUnderlay>
  )
}
