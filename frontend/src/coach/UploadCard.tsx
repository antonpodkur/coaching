import { useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router'

import { UploadIcon } from '../shared/icons'
import { useCoach } from './context'
import { type UploadProgress, dismissUpload, retryUpload } from './uploads'

const PHASE_TEXT: Record<UploadProgress['phase'], string> = {
  preparing: 'Готую завантаження…',
  uploading: 'Відео завантажується',
  finishing: 'Завершую завантаження…',
  failed: 'Не вдалося завантажити відео',
}

/** An upload running on this phone, with its progress. */
export function UploadCard({ upload, linkToExercise }: { upload: UploadProgress; linkToExercise?: boolean }) {
  const { base } = useCoach()
  const queryClient = useQueryClient()
  const percent = Math.round(upload.progress * 100)
  const failed = upload.phase === 'failed'

  return (
    <section className="upload-card" role="status" aria-label="Завантаження відео">
      <div className="upload-head">
        <span className="upload-icon">
          <UploadIcon />
        </span>
        <span className="upload-text">
          {linkToExercise ? (
            <Link to={`${base}/exercises/${upload.exerciseId}`} className="upload-name">
              {upload.exerciseName}
            </Link>
          ) : (
            <span className="upload-name">{upload.exerciseName}</span>
          )}
          <span className="upload-phase">
            {upload.notConfigured ? 'Відео ще не налаштоване на сервері' : PHASE_TEXT[upload.phase]}
            {upload.phase === 'uploading' && ` · ${percent}%`}
          </span>
        </span>
      </div>
      {!failed && (
        <div
          className="progress"
          role="progressbar"
          aria-label="Завантаження відео"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent}
        >
          <div style={{ width: `${percent}%` }} />
        </div>
      )}
      {failed ? (
        <div className="upload-actions">
          {!upload.notConfigured && (
            <button
              type="button"
              className="button small primary"
              onClick={() => retryUpload(upload.exerciseId, queryClient)}
            >
              Спробувати ще раз
            </button>
          )}
          <button
            type="button"
            className="link-button"
            onClick={() => dismissUpload(upload.exerciseId)}
          >
            {upload.notConfigured ? 'Закрити' : 'Скасувати'}
          </button>
        </div>
      ) : (
        <p className="muted small">
          Не закривай Telegram, доки завантаження не завершиться. Далі відео обробиться саме.
        </p>
      )}
    </section>
  )
}
