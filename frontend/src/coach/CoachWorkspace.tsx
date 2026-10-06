import { useMemo } from 'react'
import { Link, Route, Routes, useLocation } from 'react-router'

import type { Schemas } from '../api/client'
import { CalendarIcon, LibraryIcon, PeopleIcon } from '../shared/icons'
import { ClientEditPage } from './ClientEditPage'
import { ClientNutritionPage } from './ClientNutritionPage'
import { ClientPage } from './ClientPage'
import { ClientQuestionnairePage } from './ClientQuestionnairePage'
import { ClientWeightPage } from './ClientWeightPage'
import { ClientsPage } from './ClientsPage'
import { ExercisePage } from './ExercisePage'
import { ImportPage } from './ImportPage'
import { InvitePage } from './InvitePage'
import { LibraryPage } from './LibraryPage'
import { NewExercisePage } from './NewExercisePage'
import { ProfilePage } from './ProfilePage'
import { ReportPage } from './ReportPage'
import { WorkoutsPage } from './WorkoutsPage'
import { BuilderPage } from './builder/BuilderPage'
import { CoachContext } from './context'

interface Props {
  /** Where the workspace lives: `/app`. */
  base: string
  coach: Schemas['CoachProfile']
  onUnauthorized: () => void
  /** Browser only; inside Telegram the session belongs to her Telegram account. */
  onSignOut?: () => void
}

/**
 * The coach's workspace, laid out for a phone: pages above a bottom tab bar.
 * The same screens serve the Mini App and the browser; on a wide screen the
 * tabs move to the top. Her profile, sign-out and the rarer tools sit behind
 * the person button on the clients page.
 */
export function CoachWorkspace({ base, coach, onUnauthorized, onSignOut }: Props) {
  const context = useMemo(() => ({ base, onUnauthorized }), [base, onUnauthorized])
  const { pathname } = useLocation()
  const section = pathname.startsWith(`${base}/exercises`)
    ? 'exercises'
    : pathname === `${base}/workouts`
      ? 'workouts'
      : 'clients'
  // Sub-pages take the whole screen and go back with Telegram's back arrow.
  const subPage = ['invite', 'profile', 'import', 'clients/', 'workouts/', 'exercises/'].some(
    (page) => pathname.startsWith(`${base}/${page}`),
  )

  return (
    <CoachContext value={context}>
      <div className="coach-shell">
        <main className="coach-main">
          <Routes>
            <Route index element={<ClientsPage />} />
            <Route path="invite" element={<InvitePage />} />
            <Route path="clients/:id" element={<ClientPage />} />
            <Route path="clients/:id/edit" element={<ClientEditPage />} />
            <Route path="clients/:id/questionnaire" element={<ClientQuestionnairePage />} />
            <Route path="clients/:id/weight" element={<ClientWeightPage />} />
            <Route path="clients/:id/nutrition" element={<ClientNutritionPage />} />
            <Route path="workouts" element={<WorkoutsPage />} />
            <Route path="workouts/:id" element={<BuilderPage />} />
            <Route path="workouts/:id/report" element={<ReportPage />} />
            <Route path="exercises" element={<LibraryPage />} />
            <Route path="exercises/new" element={<NewExercisePage />} />
            <Route path="exercises/:id" element={<ExercisePage />} />
            <Route path="profile" element={<ProfilePage coach={coach} onSignOut={onSignOut} />} />
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
          </nav>
        )}
      </div>
    </CoachContext>
  )
}
