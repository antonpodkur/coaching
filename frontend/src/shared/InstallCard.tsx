import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useNavigate } from 'react-router'

import { type AppSession, SESSION_KEY } from '../app/session'
import { telegramWebApp } from '../app/telegram'
import { Collapse } from './Collapse'
import { installedApp } from './device'
import { ChevronIcon, PhoneIcon } from './icons'

const DISMISSED_KEY = 'install_card_dismissed'

/** Telegram's own browser can't install, so from Telegram the page opens in the phone's. */
function openInstallPage() {
  telegramWebApp()?.openLink(new URL('/install', window.location.origin).href)
}

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
 * and pushes the page under a finger. Put off, it folds away.
 */
export function InstallCard() {
  const queryClient = useQueryClient()
  const [dismissed, setDismissed] = useState(readDismissed)
  const webApp = telegramWebApp()
  const session = queryClient.getQueryData<AppSession>(SESSION_KEY)
  const installed =
    session?.role === 'coach' ? session.coach.app_installed : session?.client.app_installed
  const phone = webApp?.platform === 'ios' || webApp?.platform.startsWith('android')

  const dismiss = () => {
    try {
      localStorage.setItem(DISMISSED_KEY, '1')
    } catch {
      // Without storage it simply shows again next time.
    }
    setDismissed(true)
  }

  return (
    <Collapse show={!!webApp && !!phone && !installed && !dismissed}>
      <div className="prompt-card">
        <strong>Застосунок на екрані телефону</strong>
        <p className="muted small">
          Відкривається одним дотиком, без Telegram, і працює навіть без зв’язку в залі.
        </p>
        <div className="prompt-actions">
          <button
            type="button"
            className="button primary small"
            onClick={openInstallPage}
          >
            Встановити
          </button>
          <button type="button" className="link-button" onClick={dismiss}>
            Не зараз
          </button>
        </div>
      </div>
    </Collapse>
  )
}

/**
 * A quiet row on the profile that always leads to the install page, for
 * whoever put the card off, deleted the app or has a new phone. A list item
 * for a `list-group`; nothing in the installed app itself.
 */
export function InstallRow({ installedBefore }: { installedBefore: boolean }) {
  const navigate = useNavigate()
  if (installedApp()) return null

  return (
    <li>
      <button
        type="button"
        className="list-row"
        onClick={() => (telegramWebApp() ? openInstallPage() : navigate('/install'))}
      >
        <span className="list-icon" aria-hidden="true">
          <PhoneIcon />
        </span>
        <span className="list-text">
          <span className="list-title">Застосунок на телефон</span>
          <span className="muted small">
            {installedBefore
              ? 'Ще раз або на новий телефон'
              : 'Одним дотиком, без Telegram'}
          </span>
        </span>
        <ChevronIcon />
      </button>
    </li>
  )
}
