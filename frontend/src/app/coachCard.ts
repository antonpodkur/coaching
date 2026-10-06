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
