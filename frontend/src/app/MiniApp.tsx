import { useQuery } from '@tanstack/react-query'
import { useCallback, useEffect, useState } from 'react'

import { ApiError, api, setSessionToken, unwrap } from '../api/client'
import { CoachWorkspace } from '../coach/CoachWorkspace'
import { Screen } from '../shared/Screen'
import { ClientApp } from './ClientApp'
import { installSwipeBack } from './gestures'
import {
  type AppSession,
  SESSION_KEY,
  recalledSession,
  rememberSession,
  syncTimezone,
} from './session'
import { telegramWebApp } from './telegram'

const webApp = telegramWebApp()

/** What a refused sign-in means, by the backend's error code. */
const SIGN_IN_ERRORS: Record<string, [string, string]> = {
  not_invited: ['Потрібне запрошення', 'Попроси в Даші посилання-запрошення в Telegram.'],
  invite_invalid: [
    'Запрошення вже не діє',
    'Воно одноразове й діє 7 днів. Попроси в Даші нове.',
  ],
  linked_elsewhere: [
    'Акаунт уже прив’язаний',
    'Цей Telegram-акаунт уже прив’язаний до іншого профілю. Напиши Даші — вона допоможе.',
  ],
}

/**
 * Trades Telegram's `initData` for a session. The backend decides the role:
 * Dasha gets her workspace, everyone else with an invite gets their workouts.
 * Without a network, a session from the last few hours is reused, so the app
 * still opens in a gym with no signal.
 */
async function signIn(initData: string): Promise<AppSession> {
  const telegramUserId = webApp?.initDataUnsafe.user?.id
  let session: AppSession
  try {
    session = unwrap(await api.POST('/auth/telegram-webapp', { body: { init_data: initData } }))
  } catch (err) {
    const recalled = err instanceof ApiError ? null : recalledSession(telegramUserId)
    if (!recalled) throw err
    setSessionToken(recalled.token)
    return recalled
  }
  setSessionToken(session.token)
  rememberSession(session, telegramUserId)
  await syncTimezone(session)
  return session
}

export function MiniApp() {
  const [expired, setExpired] = useState(false)
  const onUnauthorized = useCallback(() => setExpired(true), [])

  useEffect(() => {
    webApp?.ready()
    webApp?.expand()
    webApp?.setHeaderColor('#121212')
    webApp?.setBackgroundColor('#121212')
  }, [])
  // On Android the system back gesture already presses Telegram's back arrow.
  useEffect(() => (webApp?.platform === 'ios' ? installSwipeBack() : undefined), [])

  const session = useQuery({
    queryKey: SESSION_KEY,
    queryFn: () => signIn(webApp?.initData ?? ''),
    enabled: webApp !== null,
    staleTime: Infinity,
    retry: false,
    // Run even when the phone says it is offline: sign-in then falls back to
    // the remembered session instead of waiting for a network.
    networkMode: 'always',
  })

  if (session.isPending || session.isFetching) {
    return (
      <Screen>
        <p className="muted">Завантаження…</p>
      </Screen>
    )
  }

  if (session.isError) {
    const code = session.error instanceof ApiError ? session.error.code : null
    const [title, text] = (code && SIGN_IN_ERRORS[code]) ?? [
      'Не вдалося увійти',
      'Закрий застосунок і відкрий його ще раз.',
    ]
    return (
      <Screen>
        <h1>{title}</h1>
        <p className="muted">{text}</p>
      </Screen>
    )
  }

  // The token ran out while the app stayed open for many hours.
  if (expired) {
    return (
      <Screen>
        <h1>Сесія завершилась</h1>
        <p className="muted">Увійди знову. Якщо не вийде — закрий застосунок і відкрий ще раз.</p>
        <button
          type="button"
          className="button primary"
          onClick={() => {
            setExpired(false)
            void session.refetch()
          }}
        >
          Увійти знову
        </button>
      </Screen>
    )
  }

  if (session.data.role === 'coach') {
    return <CoachWorkspace base="/app" onUnauthorized={onUnauthorized} />
  }
  return <ClientApp client={session.data.client} />
}
