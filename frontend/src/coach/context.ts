import { createContext, useContext } from 'react'

import { ApiError } from '../api/client'

export interface CoachContextValue {
  /** Where the workspace is mounted: `/app` in the Mini App, `/coach` in a browser. */
  base: string
  /** The session was rejected: sign out in a browser, ask to reopen in the Mini App. */
  onUnauthorized: () => void
}

export const CoachContext = createContext<CoachContextValue | null>(null)

export function useCoach(): CoachContextValue {
  const value = useContext(CoachContext)
  if (!value) throw new Error('useCoach() outside <CoachWorkspace>')
  return value
}

export const CLIENTS_KEY = ['coach-clients']

export function isUnauthorized(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401
}
