import { Link } from 'react-router'

import type { Schemas } from '../api/client'
import { Avatar } from '../shared/Avatar'
import { BackLink } from '../shared/BackLink'
import { ChevronIcon, ImportIcon } from '../shared/icons'
import { useCoach } from './context'

/** The coach's own page: who is signed in, the rarer tools, and signing out. */
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
      <header className="client-head">
        <Avatar name={coach.name} large />
        <div className="client-text">
          <h1>{coach.name}</h1>
          <span className="client-status">Тренер</span>
        </div>
      </header>

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
