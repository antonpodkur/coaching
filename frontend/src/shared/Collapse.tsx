import { AnimatePresence, type Variants, m } from 'motion/react'
import { type ReactNode, type RefObject, useRef } from 'react'

import { EASE_IOS } from './motion'

const variants: Variants = {
  // Folds into the gap above it, so the page ends up exactly as without it.
  gone: (node: RefObject<HTMLDivElement | null>) => {
    const parent = node.current?.parentElement
    const gap = parent ? parseFloat(getComputedStyle(parent).rowGap) || 0 : 0
    return { opacity: 0, height: 0, marginTop: -gap }
  },
}

/**
 * A card that can go away, such as one put off with "Не зараз": it folds up
 * and what is below glides up, instead of jumping. It is there or not from
 * the first draw; only going away is animated.
 */
export function Collapse({ show, children }: { show: boolean; children: ReactNode }) {
  const node = useRef<HTMLDivElement>(null)
  return (
    <AnimatePresence initial={false}>
      {show && (
        <m.div
          ref={node}
          className="collapse"
          custom={node}
          variants={variants}
          exit="gone"
          transition={{ duration: 0.28, ease: EASE_IOS }}
        >
          {children}
        </m.div>
      )}
    </AnimatePresence>
  )
}
