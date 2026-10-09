import { useEffect } from 'react'

import { Screen } from '../shared/Screen'
import { APP_ORIGIN } from './moved'

/**
 * The app installed from the old address: it says where the app is now, and
 * gives up its notifications, which the app installed from there takes over.
 */
export function MovedPage() {
  useEffect(() => {
    void dropNotifications().catch(() => undefined)
  }, [])

  return (
    <Screen>
      <header className="install-head">
        <img className="install-icon" src="/apple-touch-icon.png" alt="" width={64} height={64} />
        <h1>Застосунок переїхав</h1>
        <p className="muted">
          Тепер він за адресою getcoachin.app. Встанови його звідти ще раз і увійди через Telegram.
          Цю іконку потім можна видалити.
        </p>
      </header>
      <a className="button primary block" href={`${APP_ORIGIN}/install`}>
        Відкрити getcoachin.app
      </a>
    </Screen>
  )
}

/** Push to this address stops; the backend forgets the subscription on its next send. */
async function dropNotifications() {
  const registration = await navigator.serviceWorker?.getRegistration()
  const subscription = await registration?.pushManager.getSubscription()
  await subscription?.unsubscribe()
}
