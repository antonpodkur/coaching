import { useCallback, useState } from 'react'

import { setSessionToken } from '../api/client'
import { CoachLogin } from './CoachLogin'
import { CoachWorkspace } from './CoachWorkspace'
import { loadCoachToken, saveCoachToken } from './session'

/** The workspace in a normal browser, for when Dasha is at a computer. */
export function CoachArea() {
  const [token, setToken] = useState(() => {
    const saved = loadCoachToken()
    setSessionToken(saved)
    return saved
  })

  const signIn = useCallback((next: string) => {
    saveCoachToken(next)
    setSessionToken(next)
    setToken(next)
  }, [])

  const signOut = useCallback(() => {
    saveCoachToken(null)
    setSessionToken(null)
    setToken(null)
  }, [])

  if (!token) return <CoachLogin onSignedIn={signIn} />
  return <CoachWorkspace base="/coach" onUnauthorized={signOut} onSignOut={signOut} />
}
