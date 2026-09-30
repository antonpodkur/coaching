import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'

import { ApiError, api, setSessionToken, unwrap } from '../api/client'
import { Screen } from '../shared/Screen'
import { telegramWebApp } from './telegram'

const webApp = telegramWebApp()

/** Trades Telegram's `initData` for a session and records the phone's timezone. */
async function signIn(initData: string) {
  const session = unwrap(
    await api.POST('/auth/telegram-webapp', { body: { init_data: initData } }),
  )
  setSessionToken(session.token)

  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone
  if (timezone && session.client.timezone !== timezone) {
    unwrap(await api.PUT('/me/timezone', { body: { timezone } }))
  }
  return session.client
}

export function MiniApp() {
  useEffect(() => {
    webApp?.ready()
    webApp?.expand()
    webApp?.setHeaderColor('#121212')
    webApp?.setBackgroundColor('#121212')
  }, [])

  const client = useQuery({
    queryKey: ['client-session'],
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

  if (client.isPending) {
    return (
      <Screen>
        <p className="muted">Завантаження…</p>
      </Screen>
    )
  }

  if (client.isError) {
    const notInvited = client.error instanceof ApiError && client.error.code === 'not_a_client'
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

  const firstName = client.data.name.split(' ')[0]
  return (
    <Screen>
      <p className="muted">
        {new Date().toLocaleDateString('uk-UA', { weekday: 'long', day: 'numeric', month: 'long' })}
      </p>
      <h1>Привіт, {firstName}</h1>
      <p className="muted">Тут з’являться тренування від Даші.</p>
    </Screen>
  )
}
