import { useState } from 'react'

import type { Schemas } from '../api/client'
import { VideoPlayer } from './VideoPlayer'
import { formatClock } from './format'
import { CloseIcon } from './icons'

/** A client's video: of an exercise, or of their gym. */
type ClientVideo = Pick<Schemas['FormVideo'], 'status' | 'hls_url' | 'thumbnail_url' | 'length_secs'>

const STATE_TEXT: Record<ClientVideo['status'], string> = {
  uploading: 'Надсилання не завершилось',
  processing: 'Відео обробляється…',
  // Ready, but no link: the private library is not set up on this server.
  ready: 'Відео недоступне',
  failed: 'Не вдалося обробити відео',
}

/**
 * A client's video (technique or gym): its thumbnail until tapped, then the player,
 * already playing. Both fill the same frame, so opening a video moves nothing
 * on the page. The links are signed and expire, so they are always fresh from
 * the server.
 */
export function FormVideoTile({
  video,
  label,
  onDelete,
}: {
  video: ClientVideo
  label: string
  onDelete?: () => void
}) {
  const [playing, setPlaying] = useState(false)
  const ready = video.status === 'ready' && video.hls_url

  return (
    <div className="form-video">
      {ready && playing ? (
        <VideoPlayer
          src={video.hls_url ?? ''}
          poster={video.thumbnail_url ?? undefined}
          label={label}
          autoPlay
        />
      ) : ready ? (
        <button type="button" className="form-video-thumb" onClick={() => setPlaying(true)}>
          {video.thumbnail_url && <img src={video.thumbnail_url} alt="" loading="lazy" />}
          <span className="form-video-play" aria-hidden="true">
            ▶
          </span>
          <span className="sr-only">Переглянути: {label}</span>
        </button>
      ) : (
        <div className={video.status === 'failed' ? 'form-video-state failed' : 'form-video-state'}>
          {STATE_TEXT[video.status]}
        </div>
      )}
      <div className="form-video-meta">
        <span className="muted small">
          {label}
          {video.length_secs != null && video.length_secs > 0 && ` · ${formatClock(video.length_secs)}`}
        </span>
        {onDelete && (
          <button
            type="button"
            className="icon-button"
            aria-label={`Видалити: ${label}`}
            onClick={onDelete}
          >
            <CloseIcon />
          </button>
        )}
      </div>
    </div>
  )
}
