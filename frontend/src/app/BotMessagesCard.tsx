import { useState } from 'react'

import { api } from '../api/client'
import { telegramSupporting } from './telegram'

/**
 * Asks for Telegram's permission for the bot to message the client. Someone who
 * joined through an app link never pressed Start, so until they allow it there
 * are no messages about new workouts and no reminders.
 */
export function BotMessagesCard({ onAllowed }: { onAllowed: () => void }) {
  const [failed, setFailed] = useState(false)
  const webApp = telegramSupporting('6.9')
  if (!webApp) return null

  const allow = () => {
    setFailed(false)
    webApp.requestWriteAccess((granted) => {
      if (!granted) return
      // The bot then sends its welcome, pinned at the top of the chat.
      api
        .POST('/me/bot-allowed')
        .then(({ response }) => (response.ok ? onAllowed() : setFailed(true)))
        .catch(() => setFailed(true))
    })
  }

  return (
    <div className="prompt-card">
      <strong>Повідомлення від бота</strong>
      <p className="muted small">
        Дозволь боту надсилати нові тренування від Даші й нагадування в день тренування.
      </p>
      <div className="prompt-actions">
        <button type="button" className="button primary small" onClick={allow}>
          Дозволити
        </button>
      </div>
      {failed && <p className="error">Не вдалося. Спробуй ще раз.</p>}
    </div>
  )
}
