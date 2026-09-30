import { useEffect, useRef, useState } from 'react'

import { ApiError, type Schemas, api, unwrap } from '../api/client'

const BOT_USERNAME = import.meta.env.VITE_BOT_USERNAME

declare global {
  interface Window {
    onTelegramAuth?: (user: Schemas['LoginWidgetPayload']) => void
  }
}

/**
 * Telegram Login Widget. It only renders on the domain set with BotFather's
 * `/setdomain` for the bot in `VITE_BOT_USERNAME`.
 */
export function CoachLogin({ onSignedIn }: { onSignedIn: (token: string) => void }) {
  const widget = useRef<HTMLDivElement>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const host = widget.current
    if (!host || !BOT_USERNAME) return

    window.onTelegramAuth = async (user) => {
      try {
        const session = unwrap(await api.POST('/auth/telegram-login', { body: user }))
        onSignedIn(session.token)
      } catch (err) {
        setError(
          err instanceof ApiError && err.code === 'not_a_coach'
            ? 'Цей Telegram-акаунт не має доступу до кабінету.'
            : 'Не вдалося увійти. Спробуй ще раз.',
        )
      }
    }

    const script = document.createElement('script')
    script.src = 'https://telegram.org/js/telegram-widget.js?22'
    script.async = true
    script.dataset.telegramLogin = BOT_USERNAME
    script.dataset.size = 'large'
    script.dataset.radius = '12'
    script.dataset.onauth = 'onTelegramAuth(user)'
    host.appendChild(script)

    return () => {
      host.replaceChildren()
      delete window.onTelegramAuth
    }
  }, [onSignedIn])

  return (
    <main className="coach-login">
      <h1>Кабінет тренера</h1>
      <p className="muted">Увійди через Telegram.</p>
      {BOT_USERNAME ? (
        <div ref={widget} />
      ) : (
        <p className="error">VITE_BOT_USERNAME не задано — див. README.</p>
      )}
      {error && <p className="error">{error}</p>}
    </main>
  )
}
