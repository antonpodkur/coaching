import { useMemo } from 'react'
import { Link, Route, Routes, useLocation } from 'react-router'

import { CalendarIcon, ImportIcon, LibraryIcon, PeopleIcon } from '../shared/icons'
import { ClientEditPage } from './ClientEditPage'
import { ClientPage } from './ClientPage'
import { ClientsPage } from './ClientsPage'
import { ExercisePage } from './ExercisePage'
import { ImportPage } from './ImportPage'
import { InvitePage } from './InvitePage'
import { LibraryPage } from './LibraryPage'
import { NewExercisePage } from './NewExercisePage'
import { ReportPage } from './ReportPage'
import { WorkoutsPage } from './WorkoutsPage'
import { BuilderPage } from './builder/BuilderPage'
import { CoachContext } from './context'

interface Props {
  /** `/app` inside Telegram, `/coach` in a browser. */
  base: string
  onUnauthorized: () => void
  /** Browser only; inside Telegram the session belongs to her Telegram account. */
  onSignOut?: () => void
}

/**
 * Dasha's workspace, laid out for a phone: pages above a bottom tab bar. The
 * same screens serve the Mini App and the browser; on a wide screen the tabs
 * move to the top.
 */
export function CoachWorkspace({ base, onUnauthorized, onSignOut }: Props) {
  const context = useMemo(() => ({ base, onUnauthorized }), [base, onUnauthorized])
  const { pathname } = useLocation()
  const section = pathname.startsWith(`${base}/import`)
    ? 'import'
    : pathname.startsWith(`${base}/exercises`)
      ? 'exercises'
      : pathname === `${base}/workouts`
        ? 'workouts'
        : 'clients'
  // Sub-pages take the whole screen and go back with Telegram's back arrow.
  const subPage = ['invite', 'clients/', 'workouts/', 'exercises/'].some((page) =>
    pathname.startsWith(`${base}/${page}`),
  )

  return (
    <CoachContext value={context}>
      <div className="coach-shell">
        {onSignOut && (
          <header className="coach-top">
            <div className="wordmark">
              <span>Daria Khyzhniak</span>
              <span className="muted">кабінет тренера</span>
            </div>
            <button type="button" className="button small" onClick={onSignOut}>
              Вийти
            </button>
          </header>
        )}
        <main className="coach-main">
          <Routes>
            <Route index element={<ClientsPage />} />
            <Route path="invite" element={<InvitePage />} />
            <Route path="clients/:id" element={<ClientPage />} />
            <Route path="clients/:id/edit" element={<ClientEditPage />} />
            <Route path="workouts" element={<WorkoutsPage />} />
            <Route path="workouts/:id" element={<BuilderPage />} />
            <Route path="workouts/:id/report" element={<ReportPage />} />
            <Route path="exercises" element={<LibraryPage />} />
            <Route path="exercises/new" element={<NewExercisePage />} />
            <Route path="exercises/:id" element={<ExercisePage />} />
            <Route path="import" element={<ImportPage />} />
          </Routes>
        </main>
        {!subPage && (
          <nav className="tab-bar" aria-label="Розділи">
            <Link to={base} aria-current={section === 'clients' ? 'page' : undefined}>
              <PeopleIcon size={22} />
              Клієнти
            </Link>
            <Link to={`${base}/workouts`} aria-current={section === 'workouts' ? 'page' : undefined}>
              <CalendarIcon size={22} />
              Тренування
            </Link>
            <Link
              to={`${base}/exercises`}
              aria-current={section === 'exercises' ? 'page' : undefined}
            >
              <LibraryIcon size={22} />
              Вправи
            </Link>
            <Link to={`${base}/import`} aria-current={section === 'import' ? 'page' : undefined}>
              <ImportIcon size={22} />
              Імпорт
            </Link>
          </nav>
        )}
      </div>
    </CoachContext>
  )
}
