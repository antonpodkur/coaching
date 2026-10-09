import { useQueryClient } from '@tanstack/react-query'
import { type ChangeEvent, useState } from 'react'
import { Link } from 'react-router'

import { ApiError, type Schemas } from '../api/client'
import { Avatar } from '../shared/Avatar'
import { BackLink } from '../shared/BackLink'
import { InstallRow } from '../shared/InstallCard'
import { confirmAction } from '../shared/dialogs'
import { ChevronIcon, ImportIcon } from '../shared/icons'
import { removeCoachAvatar, showCoachAvatar, uploadCoachAvatar } from './coachAvatar'
import { useCoach } from './context'

/** The coach's own page: her photo, the rarer tools, the app's install page, and signing out. */
export function ProfilePage({
  coach,
  onSignOut,
}: {
  coach: Schemas['CoachProfile']
  /** Outside Telegram only; inside it the session is her Telegram account's. */
  onSignOut?: () => void
}) {
  const { base } = useCoach()

  return (
    <section className="page">
      <BackLink to={base} label="Клієнти" />
      <PhotoSection coach={coach} />

      <ul className="list-group">
        <li>
          <Link className="list-row" to={`${base}/import`}>
            <span className="list-icon" aria-hidden="true">
              <ImportIcon size={18} />
            </span>
            <span className="list-text">
              <span className="list-title">Імпорт плану з Telegram</span>
              <span className="muted small">Перенести старий план клієнта</span>
            </span>
            <ChevronIcon />
          </Link>
        </li>
        <InstallRow installedBefore={coach.app_installed} />
      </ul>

      <dl className="facts">
        <div className="fact">
          <dt>Часовий пояс</dt>
          <dd>{coach.timezone}</dd>
        </div>
      </dl>
      <p className="muted small">
        За ним приходить вечірній підсумок. Береться з телефона, коли відкриваєш застосунок.
      </p>

      {onSignOut && (
        <button type="button" className="link-button sign-out" onClick={onSignOut}>
          Вийти з акаунта
        </button>
      )}
    </section>
  )
}

/** Her photo, which clients see next to her workouts and comments. */
function PhotoSection({ coach }: { coach: Schemas['CoachProfile'] }) {
  const queryClient = useQueryClient()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const url = coach.avatar_url ?? null

  const choose = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (!file) return
    setError(null)
    setBusy(true)
    try {
      showCoachAvatar(queryClient, await uploadCoachAvatar(file))
    } catch (err) {
      console.error('coach photo upload failed', err)
      setError(
        err instanceof ApiError && err.code === 'photos_not_configured'
          ? 'Фото ще не налаштовані.'
          : 'Не вдалося додати фото. Перевір зв’язок і спробуй ще раз.',
      )
    } finally {
      setBusy(false)
    }
  }

  const remove = async () => {
    if (!(await confirmAction('Прибрати фото? Клієнти бачитимуть твої ініціали.'))) return
    setError(null)
    setBusy(true)
    try {
      await removeCoachAvatar()
      showCoachAvatar(queryClient, null)
    } catch {
      setError('Не вдалося прибрати фото. Перевір зв’язок і спробуй ще раз.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="profile-photo" aria-label="Фото профілю">
      <Avatar name={coach.name} url={url} large />
      <div className="profile-photo-text">
        <h1>{coach.name}</h1>
        <span className="muted small">
          {url
            ? 'Це фото бачать твої клієнти поруч із тренуваннями й коментарями.'
            : 'Додай фото — клієнти бачитимуть його поруч із тренуваннями й коментарями.'}
        </span>
        <div className="profile-photo-actions">
          <label className="button small file-button">
            {busy ? 'Зберігаю…' : url ? 'Змінити фото' : 'Додати фото'}
            <input
              type="file"
              accept="image/*"
              disabled={busy}
              onChange={(event) => void choose(event)}
            />
          </label>
          {url && (
            <button
              type="button"
              className="link-button"
              disabled={busy}
              onClick={() => void remove()}
            >
              Прибрати
            </button>
          )}
        </div>
      </div>
      {error && <p className="error">{error}</p>}
    </section>
  )
}
