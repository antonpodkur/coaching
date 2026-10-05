import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { Route, Routes } from 'react-router'

import type { Schemas } from '../api/client'
import { holdClosing } from '../shared/closingGuard'
import { ExerciseScreen } from './ExerciseScreen'
import { FinishPage } from './FinishPage'
import { HomePage } from './HomePage'
import { NutritionPage } from './NutritionPage'
import { QuestionnairePage } from './QuestionnairePage'
import { WeightPage } from './WeightPage'
import { WorkoutPage } from './WorkoutPage'
import { flush, onDrained, useOutboxStatus } from './outbox'
import { MY_WORKOUTS_KEY } from './workouts'

/** The client's side of the Mini App: today, a workout, an exercise, the report. */
export function ClientApp({ client }: { client: Schemas['ClientProfile'] }) {
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

  return (
    <>
      <Routes>
        <Route index element={<HomePage client={client} />} />
        <Route path="workouts/:id" element={<WorkoutPage />} />
        <Route path="workouts/:id/exercises/:exerciseId" element={<ExerciseScreen />} />
        <Route path="workouts/:id/finish" element={<FinishPage />} />
        <Route path="questionnaire" element={<QuestionnairePage />} />
        <Route path="weight" element={<WeightPage />} />
        <Route path="nutrition" element={<NutritionPage />} />
      </Routes>
      <OfflineNote />
    </>
  )
}

function OfflineNote() {
  const { waiting, failing } = useOutboxStatus()
  if (waiting === 0 || !failing) return null
  return (
    <p className="offline-note" role="status">
      Немає зв’язку. Зміни збережено на телефоні й надішлються самі.
    </p>
  )
}
