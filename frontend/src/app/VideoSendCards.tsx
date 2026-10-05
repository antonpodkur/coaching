import { CloseIcon } from '../shared/icons'
import { type VideoSend, dismissSend } from './videoSends'

/** Videos on their way to Dasha: progress, or why one failed. */
export function VideoSendCards({ sends }: { sends: VideoSend[] }) {
  return sends.map((send) => (
    <div key={send.key} className={send.phase === 'failed' ? 'video-send failed' : 'video-send'}>
      <div className="video-send-head">
        <span className="small">
          {send.phase === 'failed'
            ? send.error
            : send.phase === 'uploading'
              ? `Надсилаю відео… ${Math.round(send.progress * 100)}%`
              : 'Готую відео…'}
        </span>
        {send.phase === 'failed' && (
          <button
            type="button"
            className="icon-button"
            aria-label="Закрити"
            onClick={() => dismissSend(send.key)}
          >
            <CloseIcon />
          </button>
        )}
      </div>
      {send.phase !== 'failed' && (
        <div
          className="progress"
          role="progressbar"
          aria-valuenow={Math.round(send.progress * 100)}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div style={{ width: `${Math.round(send.progress * 100)}%` }} />
        </div>
      )}
    </div>
  ))
}
