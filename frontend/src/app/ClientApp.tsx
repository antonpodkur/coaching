import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { Route, Routes } from 'react-router'

import type { Schemas } from '../api/client'
import { BottomNote } from '../shared/BottomNote'
import { SwipeUnderlay } from '../shared/SwipeUnderlay'
import { holdClosing } from '../shared/closingGuard'
import { ExerciseScreen } from './ExerciseScreen'
import { FinishPage } from './FinishPage'
import { HomePage } from './HomePage'
import { NutritionPage } from './NutritionPage'
import { QuestionnairePage } from './QuestionnairePage'
import { WeightPage } from './WeightPage'
import { WorkoutPage } from './WorkoutPage'
import { CoachCardContext } from './coachCard'
import { flush, onDrained, useOutboxStatus } from './outbox'
import { MY_WORKOUTS_KEY } from './workouts'

/**
 * The client's side of the app: today, a workout, an exercise, the report.
 * `onSignOut` is there outside Telegram, where the session is kept on the phone.
 */
export function ClientApp({
  client,
  onSignOut,
}: {
  client: Schemas['ClientProfile']
  onSignOut?: () => void
}) {
  const queryClient = useQueryClient()
  const { waiting } = useOutboxStatus()

  // Send what an earlier session left queued, and refresh once all of it is in.
  useEffect(() => {
    void flush()
    return onDrained(() => void queryClient.invalidateQueries({ queryKey: MY_WORKOUTS_KEY }))
  }, [queryClient])

  // Closing the app now would leave logged sets on the phone until next time.
  useEffect(() => {
    holdClosing('outbox', waiting > 0)
    return () => holdClosing('outbox', false)
  }, [waiting])

  // A session saved before the coach card existed has none until it renews.
  const coach = (client as Partial<Schemas['ClientProfile']>).coach ?? null

  const routes = (
    <>
      <Route index element={<HomePage client={client} />} />
      <Route path="workouts/:id" element={<WorkoutPage />} />
      <Route path="workouts/:id/exercises/:exerciseId" element={<ExerciseScreen />} />
      <Route path="workouts/:id/finish" element={<FinishPage />} />
      <Route
        path="questionnaire"
        element={<QuestionnairePage client={client} onSignOut={onSignOut} />}
      />
      <Route path="weight" element={<WeightPage />} />
      <Route path="nutrition" element={<NutritionPage />} />
    </>
  )

  return (
    <CoachCardContext value={coach}>
      <Routes>{routes}</Routes>
      <SwipeUnderlay render={(location) => <Routes location={location}>{routes}</Routes>} />
      <OfflineNote />
    </CoachCardContext>
  )
}

function OfflineNote() {
  const { waiting, failing } = useOutboxStatus()
  return (
    <BottomNote show={waiting > 0 && failing} className="offline-note">
      Немає зв’язку. Зміни збережено на телефоні й надішлються самі.
    </BottomNote>
  )
}
