import { createContext, useContext } from 'react'

import type { Schemas } from '../api/client'

export type CoachCard = Schemas['CoachCard']

/**
 * The client's coach, as their screens show her next to workouts and
 * comments. `null` in a session saved on the phone before the app knew it,
 * until that session renews.
 */
export const CoachCardContext = createContext<CoachCard | null>(null)

export function useCoachCard(): CoachCard | null {
  return useContext(CoachCardContext)
}

/**
 * The coach's name for a sentence. Ukrainian changes a name by case and a
 * past-tense verb by gender, so texts use it only as the subject of a verb in
 * the present or future ("Даша побачить"), which fits any coach.
 */
export function useCoachName(): string {
  return useCoachCard()?.name ?? 'Тренер'
}
