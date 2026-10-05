import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router'

import { ApiError, api, setSessionToken, unwrap } from '../api/client'
import { CoachWorkspace } from '../coach/CoachWorkspace'
import { Screen } from '../shared/Screen'
import { installedOnIphone } from '../shared/device'
import { stopPush, syncPush } from '../shared/push'
import { ClientApp } from './ClientApp'
import { SignIn } from './SignIn'
import { installSwipeBack } from './gestures'
import {
  type AppSession,
  SESSION_KEY,
  noteInstalled,
  oldCoachToken,
  saveSession,
  savedSession,
  syncTimezone,
} from './session'

/** An app left open renews its session when it comes back after this long. */
const RENEW_AFTER_MS = 12 * 60 * 60 * 1000

function keep(session: AppSession) {
  setSessionToken(session.token)
  saveSession(session)
  void syncTimezone(session)
  noteInstalled(session)
  void syncPush()
}

/**
 * Renews the saved session. `null` once the server has ended it: the client
 * was archived, or the app was not opened for a month. Without a network it
 * throws, and the saved session stays in use.
 */
async function renew(): Promise<AppSession | null> {
  try {
    const session = unwrap(await api.POST('/auth/refresh'))
    keep(session)
    return session
  } catch (err) {
    if (!(err instanceof ApiError && err.status === 401)) throw err
    setSessionToken(null)
    saveSession(null)
    return null
  }
}

/**
 * The app outside Telegram: installed on the home screen, or in a browser. It
 * opens at once with the session saved here, also with no signal, and renews
 * it in the background; without one it shows the bot sign-in.
 */
export function StandaloneApp() {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [saved] = useState(() => {
    const session = savedSession()
    const token = session?.token ?? oldCoachToken()
    setSessionToken(token)
    return { session, token }
  })
  const [signedOut, setSignedOut] = useState(saved.token === null)

  const session = useQuery({
    queryKey: SESSION_KEY,
    queryFn: renew,
    initialData: saved.session ?? undefined,
    // Stale from the start, so it renews as the app opens.
    initialDataUpdatedAt: 0,
    enabled: !signedOut,
    staleTime: RENEW_AFTER_MS,
    refetchOnWindowFocus: true,
    retry: false,
    networkMode: 'always',
  })

  useEffect(() => (installedOnIphone() ? installSwipeBack() : undefined), [])

  const signIn = useCallback(
    (next: AppSession) => {
      keep(next)
      queryClient.setQueryData(SESSION_KEY, next)
      setSignedOut(false)
    },
    [queryClient],
  )

  const signOut = useCallback(() => {
    const token = savedSession()?.token
    if (token) void stopPush(token)
    setSessionToken(null)
    saveSession(null)
    setSignedOut(true)
    queryClient.removeQueries({ queryKey: SESSION_KEY })
    // Whoever signs in next starts at their home screen.
    void navigate('/app', { replace: true })
  }, [queryClient, navigate])

  if (signedOut || session.data === null) return <SignIn onSignedIn={signIn} />

  // Only an older token, still being renewed.
  if (!session.data) {
    return (
      <Screen>
        {session.isError ? (
          <>
            <p className="muted">Не вдалося увійти. Перевір зв’язок.</p>
            <button type="button" className="button primary" onClick={() => void session.refetch()}>
              Спробувати ще раз
            </button>
          </>
        ) : (
          <p className="muted">Завантаження…</p>
        )}
      </Screen>
    )
  }

  if (session.data.role === 'coach') {
    return <CoachWorkspace base="/app" onUnauthorized={signOut} onSignOut={signOut} />
  }
  return <ClientApp client={session.data.client} onSignOut={signOut} />
}
