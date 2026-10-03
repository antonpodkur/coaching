import { useEffect, useState } from 'react'

import { type HomeScreenStatus, telegramSupporting } from '../app/telegram'

const DISMISSED_KEY = 'home_screen_card_dismissed'

function readDismissed(): boolean {
  try {
    return localStorage.getItem(DISMISSED_KEY) === '1'
  } catch {
    return false
  }
}

/**
 * Offers an icon on the phone's home screen, so the app opens in one tap
 * instead of through the chat. Shown only where Telegram supports it and the
 * icon is not there yet; "Не зараз" hides it on this phone.
 */
export function HomeScreenCard() {
  const [status, setStatus] = useState<HomeScreenStatus | null>(null)
  const [dismissed, setDismissed] = useState(readDismissed)

  useEffect(() => {
    const webApp = telegramSupporting('8.0')
    if (!webApp || dismissed) return
    webApp.checkHomeScreenStatus(setStatus)
    const added = () => setStatus('added')
    webApp.onEvent('homeScreenAdded', added)
    return () => webApp.offEvent('homeScreenAdded', added)
  }, [dismissed])

  if (dismissed || status !== 'missed') return null

  const dismiss = () => {
    try {
      localStorage.setItem(DISMISSED_KEY, '1')
    } catch {
      // Without storage it simply shows again next time.
    }
    setDismissed(true)
  }

  return (
    <div className="prompt-card">
      <strong>Іконка на екрані телефону</strong>
      <p className="muted small">Відкривай застосунок одним дотиком, без пошуку чату в Telegram.</p>
      <div className="prompt-actions">
        <button
          type="button"
          className="button primary small"
          onClick={() => telegramSupporting('8.0')?.addToHomeScreen()}
        >
          Додати
        </button>
        <button type="button" className="link-button" onClick={dismiss}>
          Не зараз
        </button>
      </div>
    </div>
  )
}
