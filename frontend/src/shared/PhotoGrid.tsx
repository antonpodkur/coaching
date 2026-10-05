import { useState } from 'react'

import { PhotoViewer } from './PhotoViewer'
import { CloseIcon } from './icons'

/**
 * Photos as square tiles, three to a row; a tap opens them full screen.
 * `pending` adds placeholder tiles for photos still on their way.
 */
export function PhotoGrid({
  photos,
  pending = 0,
  onDelete,
}: {
  photos: { id: string; url: string }[]
  pending?: number
  onDelete?: (id: string) => void
}) {
  const [open, setOpen] = useState<number | null>(null)
  if (photos.length === 0 && pending === 0) return null

  return (
    <>
      <ul className="photo-grid">
        {photos.map((photo, index) => (
          <li key={photo.id} className="photo-tile">
            <button type="button" onClick={() => setOpen(index)} aria-label={`Фото ${index + 1}`}>
              <img src={photo.url} alt="" loading="lazy" />
            </button>
            {onDelete && (
              <button
                type="button"
                className="photo-tile-delete"
                aria-label={`Видалити фото ${index + 1}`}
                onClick={() => onDelete(photo.id)}
              >
                <CloseIcon size={14} />
              </button>
            )}
          </li>
        ))}
        {Array.from({ length: pending }, (_, index) => (
          <li key={`pending-${index}`} className="photo-tile pending" aria-label="Фото завантажується" />
        ))}
      </ul>
      {open !== null && (
        <PhotoViewer photos={photos} start={open} onClose={() => setOpen(null)} />
      )}
    </>
  )
}
