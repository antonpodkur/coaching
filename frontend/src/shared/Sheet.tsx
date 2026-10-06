import { type PanInfo, m, useDragControls } from 'motion/react'
import { type ReactNode, useEffect, useRef } from 'react'

import { CloseIcon } from './icons'
import { EASE_IOS } from './motion'

interface Props {
  title: string
  /** For `aria-labelledby`; unique on the page. */
  titleId: string
  onClose: () => void
  /** As tall as its content instead of most of the screen. */
  fit?: boolean
  children: ReactNode
}

/** Pulled down this far, or flicked down this fast (px/s), the sheet closes. */
const CLOSE_DISTANCE = 120
const CLOSE_SPEED = 600
/** The page behind, as `.sheet-backdrop` dims it. */
const BACKDROP_DIM = 'rgba(8, 8, 8, 0.72)'
const BACKDROP_CLEAR = 'rgba(8, 8, 8, 0)'

/**
 * A bottom sheet, as on an iPhone: it slides up over a darkening page and
 * closes with ✕, a tap outside, Escape, or pulled down by its top. Put it in
 * an `AnimatePresence` so it can slide away.
 */
export function Sheet({ title, titleId, onClose, fit = false, children }: Props) {
  const dialog = useRef<HTMLDivElement>(null)
  const drag = useDragControls()

  useEffect(() => {
    const node = dialog.current
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    node?.addEventListener('keydown', onKey)
    return () => node?.removeEventListener('keydown', onKey)
  }, [onClose])

  const release = (_: unknown, { offset, velocity }: PanInfo) => {
    if (offset.y > CLOSE_DISTANCE || velocity.y > CLOSE_SPEED) onClose()
  }

  return (
    <m.div
      className="sheet-backdrop"
      onClick={onClose}
      // The dimming fades, not the backdrop: the sheet on it stays solid.
      initial={{ backgroundColor: BACKDROP_CLEAR }}
      animate={{ backgroundColor: BACKDROP_DIM }}
      exit={{ backgroundColor: BACKDROP_CLEAR }}
      transition={{ duration: 0.25 }}
    >
      <m.div
        ref={dialog}
        className={fit ? 'sheet fit' : 'sheet'}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(event) => event.stopPropagation()}
        initial={{ y: '100%' }}
        animate={{ y: 0 }}
        exit={{ y: '100%' }}
        transition={{ duration: 0.36, ease: EASE_IOS }}
        drag="y"
        dragControls={drag}
        dragListener={false}
        dragConstraints={{ top: 0, bottom: 0 }}
        dragElastic={{ top: 0.04, bottom: 0.9 }}
        onDragEnd={release}
      >
        {/* The top is what pulls it down; anything below scrolls as usual. */}
        <div className="sheet-grip" onPointerDown={(event) => drag.start(event)}>
          <span className="sheet-handle" aria-hidden="true" />
          <div className="sheet-head">
            <h2 id={titleId}>{title}</h2>
            <button
              type="button"
              className="icon-button filled"
              aria-label="Закрити"
              onClick={onClose}
            >
              <CloseIcon />
            </button>
          </div>
        </div>
        {children}
      </m.div>
    </m.div>
  )
}
