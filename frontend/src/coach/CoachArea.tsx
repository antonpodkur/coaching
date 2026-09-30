import { useCallback, useState } from 'react'

import { setSessionToken } from '../api/client'
import { CoachLogin } from './CoachLogin'
import { ImportPage } from './ImportPage'
import { loadCoachToken, saveCoachToken } from './session'

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

  return (
    <div className="coach">
      <header className="coach-header">
        <div className="wordmark">
          <span>Daria Khyzhniak</span>
          <span className="muted">кабінет тренера</span>
        </div>
        <button type="button" className="button" onClick={signOut}>
          Вийти
        </button>
      </header>
      <ImportPage onUnauthorized={signOut} />
    </div>
  )
}
