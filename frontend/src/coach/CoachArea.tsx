import { useCallback, useState } from 'react'
import { NavLink, Route, Routes } from 'react-router'

import { setSessionToken } from '../api/client'
import { ClientsPage } from './ClientsPage'
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
        <nav className="coach-nav" aria-label="Розділи">
          <NavLink to="/coach" end>
            Клієнти
          </NavLink>
          <NavLink to="/coach/import">Імпорт</NavLink>
        </nav>
        <button type="button" className="button" onClick={signOut}>
          Вийти
        </button>
      </header>
      <Routes>
        <Route index element={<ClientsPage onUnauthorized={signOut} />} />
        <Route path="import" element={<ImportPage onUnauthorized={signOut} />} />
      </Routes>
    </div>
  )
}
