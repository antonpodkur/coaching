import { type TouchEvent, useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import { ChevronIcon, CloseIcon } from './icons'

const MAX_SCALE = 4
const DOUBLE_TAP_SCALE = 2.5
const DOUBLE_TAP_MS = 300
/** A sideways swipe this long shows the next or previous photo. */
const SWIPE_PX = 60

interface View {
  scale: number
  x: number
  y: number
}

const FIT: View = { scale: 1, x: 0, y: 0 }

/** What the fingers did when the gesture started, in px from the screen's centre. */
type Gesture =
  | { kind: 'pinch'; distance: number; mid: Point; from: View }
  | { kind: 'drag'; start: Point; from: View; moved: Point }

interface Point {
  x: number
  y: number
}

/**
 * Photos full screen, one at a time: pinch or double-tap to zoom and drag to
 * look around (the page itself does not zoom), swipe sideways for the next one.
 */
export function PhotoViewer({
  photos,
  start,
  onClose,
}: {
  photos: { id: string; url: string }[]
  start: number
  onClose: () => void
}) {
  const [index, setIndex] = useState(start)
  const [view, setView] = useState<View>(FIT)
  const gesture = useRef<Gesture | null>(null)
  const lastTap = useRef(0)
  const image = useRef<HTMLImageElement>(null)
  const photo = photos[index]

  const show = (next: number) => {
    if (next < 0 || next >= photos.length) return
    setIndex(next)
    setView(FIT)
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
      if (event.key === 'ArrowLeft') show(index - 1)
      if (event.key === 'ArrowRight') show(index + 1)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  if (!photo) return null

  /** A touch's position relative to the centre of the screen. */
  const fromCentre = (touch: { clientX: number; clientY: number }): Point => ({
    x: touch.clientX - window.innerWidth / 2,
    y: touch.clientY - window.innerHeight / 2,
  })
  /** Keeps a zoomed photo from being dragged off the screen. */
  const clamp = (next: View): View => {
    const box = image.current
    if (!box || next.scale <= 1) return FIT
    const width = box.offsetWidth
    const height = box.offsetHeight
    const limitX = Math.max(0, (width * next.scale - window.innerWidth) / 2)
    const limitY = Math.max(0, (height * next.scale - window.innerHeight) / 2)
    return {
      scale: next.scale,
      x: Math.min(limitX, Math.max(-limitX, next.x)),
      y: Math.min(limitY, Math.max(-limitY, next.y)),
    }
  }

  const onTouchStart = (event: TouchEvent) => {
    const [a, b] = [event.touches.item(0), event.touches.item(1)]
    if (a && b) {
      const pa = fromCentre(a)
      const pb = fromCentre(b)
      gesture.current = {
        kind: 'pinch',
        distance: Math.hypot(pa.x - pb.x, pa.y - pb.y),
        mid: { x: (pa.x + pb.x) / 2, y: (pa.y + pb.y) / 2 },
        from: view,
      }
      return
    }
    if (!a) return
    const point = fromCentre(a)
    // A second tap soon after the first zooms in there, or back out.
    if (event.timeStamp - lastTap.current < DOUBLE_TAP_MS) {
      lastTap.current = 0
      gesture.current = null
      setView(
        view.scale > 1
          ? FIT
          : clamp({
              scale: DOUBLE_TAP_SCALE,
              x: -point.x * (DOUBLE_TAP_SCALE - 1),
              y: -point.y * (DOUBLE_TAP_SCALE - 1),
            }),
      )
      return
    }
    lastTap.current = event.timeStamp
    gesture.current = { kind: 'drag', start: point, from: view, moved: { x: 0, y: 0 } }
  }

  const onTouchMove = (event: TouchEvent) => {
    const current = gesture.current
    const [a, b] = [event.touches.item(0), event.touches.item(1)]
    if (!current || !a) return
    if (current.kind === 'pinch' && b) {
      const pa = fromCentre(a)
      const pb = fromCentre(b)
      const mid = { x: (pa.x + pb.x) / 2, y: (pa.y + pb.y) / 2 }
      const scale = Math.min(
        MAX_SCALE,
        Math.max(1, current.from.scale * (Math.hypot(pa.x - pb.x, pa.y - pb.y) / current.distance)),
      )
      // The point that was under the fingers stays under them.
      const ratio = scale / current.from.scale
      setView({
        scale,
        x: mid.x - (current.mid.x - current.from.x) * ratio,
        y: mid.y - (current.mid.y - current.from.y) * ratio,
      })
      return
    }
    if (current.kind === 'drag') {
      const point = fromCentre(a)
      const moved = { x: point.x - current.start.x, y: point.y - current.start.y }
      gesture.current = { ...current, moved }
      if (current.from.scale > 1) {
        setView(clamp({ ...current.from, x: current.from.x + moved.x, y: current.from.y + moved.y }))
      }
    }
  }

  const onTouchEnd = () => {
    const current = gesture.current
    gesture.current = null
    if (!current) return
    if (current.kind === 'pinch') {
      setView((latest) => clamp(latest))
      return
    }
    // Not zoomed in: a sideways swipe shows the next or previous photo.
    const { x, y } = current.moved
    if (current.from.scale === 1 && Math.abs(x) > SWIPE_PX && Math.abs(x) > Math.abs(y)) {
      show(x < 0 ? index + 1 : index - 1)
    }
  }

  return createPortal(
    <div className="photo-viewer" role="dialog" aria-modal="true" aria-label="Фото">
      <div
        className="photo-viewer-stage"
        onTouchStart={onTouchStart}
        onTouchMove={onTouchMove}
        onTouchEnd={onTouchEnd}
        onTouchCancel={onTouchEnd}
      >
        <img
          ref={image}
          key={photo.id}
          src={photo.url}
          alt={`Фото ${index + 1} з ${photos.length}`}
          draggable={false}
          style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` }}
        />
      </div>
      <button type="button" className="photo-viewer-close" aria-label="Закрити" onClick={onClose}>
        <CloseIcon size={20} />
      </button>
      {photos.length > 1 && (
        <>
          <span className="photo-viewer-count">
            {index + 1} / {photos.length}
          </span>
          <button
            type="button"
            className="photo-viewer-step previous"
            aria-label="Попереднє фото"
            disabled={index === 0}
            onClick={() => show(index - 1)}
          >
            <ChevronIcon />
          </button>
          <button
            type="button"
            className="photo-viewer-step"
            aria-label="Наступне фото"
            disabled={index === photos.length - 1}
            onClick={() => show(index + 1)}
          >
            <ChevronIcon />
          </button>
        </>
      )}
    </div>,
    document.body,
  )
}
