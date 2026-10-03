import { useState } from 'react'

import { telegramSupporting } from '../app/telegram'
import { useHomeScreenStatus } from './homeScreen'

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
 *
 * It sits above the page, so it is shown only if the answer was in when the
 * page drew: a card that pops in later pushes the page down under a finger.
 */
export function HomeScreenCard() {
  const current = useHomeScreenStatus()
  const [missedAtStart] = useState(current === 'missed')
  const [dismissed, setDismissed] = useState(readDismissed)

  if (dismissed || !missedAtStart || current !== 'missed') return null

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
