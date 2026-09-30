import { useQuery } from '@tanstack/react-query'
import { useCallback, useEffect, useState } from 'react'

import { ApiError, type Schemas, api, setSessionToken, unwrap } from '../api/client'
import { CoachWorkspace } from '../coach/CoachWorkspace'
import { Screen } from '../shared/Screen'
import { ClientHome } from './ClientHome'
import { telegramWebApp } from './telegram'

const webApp = telegramWebApp()

/**
 * Trades Telegram's `initData` for a session. The backend decides the role:
 * Dasha gets her workspace, everyone else with an invite gets their workouts.
 */
async function signIn(initData: string): Promise<Schemas['MiniAppSession']> {
  const session = unwrap(
    await api.POST('/auth/telegram-webapp', { body: { init_data: initData } }),
  )
  setSessionToken(session.token)

  if (session.role === 'client') {
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone
    if (timezone && session.client.timezone !== timezone) {
      unwrap(await api.PUT('/me/timezone', { body: { timezone } }))
    }
  }
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

  const session = useQuery({
    queryKey: ['mini-app-session'],
    queryFn: () => signIn(webApp?.initData ?? ''),
    enabled: webApp !== null,
    staleTime: Infinity,
    retry: false,
  })

  if (!webApp) {
    return (
      <Screen>
        <h1>Відкрий у Telegram</h1>
        <p className="muted">Застосунок відкривається кнопкою в чаті з ботом.</p>
      </Screen>
    )
  }

  if (session.isPending || session.isFetching) {
    return (
      <Screen>
        <p className="muted">Завантаження…</p>
      </Screen>
    )
  }

  if (session.isError) {
    const notInvited = session.error instanceof ApiError && session.error.code === 'not_invited'
    return (
      <Screen>
        <h1>{notInvited ? 'Потрібне запрошення' : 'Не вдалося увійти'}</h1>
        <p className="muted">
          {notInvited
            ? 'Попроси в Даші посилання-запрошення в Telegram.'
            : 'Закрий застосунок і відкрий його ще раз.'}
        </p>
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
  return <ClientHome client={session.data.client} />
}
