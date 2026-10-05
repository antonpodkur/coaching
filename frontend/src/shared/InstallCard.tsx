import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'

import { type AppSession, SESSION_KEY } from '../app/session'
import { telegramWebApp } from '../app/telegram'

const DISMISSED_KEY = 'install_card_dismissed'

function readDismissed(): boolean {
  try {
    return localStorage.getItem(DISMISSED_KEY) === '1'
  } catch {
    return false
  }
}

/**
 * Inside Telegram on a phone, offers the app itself on the home screen: it
 * opens in one tap, without Telegram, and works with no signal. "Встановити"
 * opens the install page in the phone's browser. Hidden for anyone already
 * using the installed app on any phone; "Не зараз" hides it on this one.
 *
 * Everything it needs is known when the page draws, so it never pops in late
 * and pushes the page under a finger.
 */
export function InstallCard() {
  const queryClient = useQueryClient()
  const [dismissed, setDismissed] = useState(readDismissed)
  const webApp = telegramWebApp()
  const session = queryClient.getQueryData<AppSession>(SESSION_KEY)
  const installed =
    session?.role === 'coach' ? session.coach.app_installed : session?.client.app_installed
  const phone = webApp?.platform === 'ios' || webApp?.platform.startsWith('android')

  if (!webApp || !phone || installed || dismissed) return null

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
      <strong>Застосунок на екрані телефону</strong>
      <p className="muted small">
        Відкривається одним дотиком, без Telegram, і працює навіть без зв’язку в залі.
      </p>
      <div className="prompt-actions">
        <button
          type="button"
          className="button primary small"
          onClick={() => webApp.openLink(new URL('/install', window.location.origin).href)}
        >
          Встановити
        </button>
        <button type="button" className="link-button" onClick={dismiss}>
          Не зараз
        </button>
      </div>
    </div>
  )
}
