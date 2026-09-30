import { useMutation, useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'

import { api, unwrap } from '../api/client'

const POLL_INTERVAL_MS = 2000

/**
 * Coach sign-in confirmed through the bot: open the link, press Confirm in
 * Telegram, and this page collects the session.
 */
export function CoachLogin({ onSignedIn }: { onSignedIn: (token: string) => void }) {
  const start = useMutation({
    mutationFn: async () => unwrap(await api.POST('/auth/bot-login')),
  })
  const login = start.data

  const result = useQuery({
    queryKey: ['bot-login', login?.poll_secret],
    queryFn: async () =>
      unwrap(
        await api.POST('/auth/bot-login/poll', {
          body: { poll_secret: login?.poll_secret ?? '' },
        }),
      ),
    enabled: login !== undefined,
    refetchInterval: (query) =>
      query.state.data?.status === 'pending' ? POLL_INTERVAL_MS : false,
    // Confirming means switching to Telegram, which hides this page (on a phone
    // it may be suspended entirely): keep polling while hidden, and check again
    // the moment she comes back.
    refetchIntervalInBackground: true,
    refetchOnWindowFocus: true,
    retry: false,
    gcTime: 0,
  })

  const status = result.data?.status
  useEffect(() => {
    if (result.data?.status === 'approved') onSignedIn(result.data.token)
  }, [result.data, onSignedIn])

  const restart = () => {
    start.reset()
    start.mutate()
  }

  return (
    <main className="coach-login">
      <h1>Кабінет тренера</h1>

      {!login && (
        <>
          <p className="muted">Вхід підтверджується в Telegram, у чаті з ботом.</p>
          <button
            type="button"
            className="button primary"
            disabled={start.isPending}
            onClick={() => start.mutate()}
          >
            {start.isPending ? 'Готую вхід…' : 'Увійти через Telegram'}
          </button>
          {start.isError && <p className="error">Не вдалося почати вхід. Спробуй ще раз.</p>}
        </>
      )}

      {login && (status === undefined || status === 'pending') && (
        <>
          <p className="muted">Код для перевірки</p>
          <p className="login-code" aria-live="polite">
            {login.display_code}
          </p>
          <a className="button primary" href={login.bot_url} target="_blank" rel="noreferrer">
            Відкрити Telegram
          </a>
          <p className="muted login-hint">
            Натисни Start і підтверди вхід. У боті має бути той самий код. Чекаю підтвердження…
          </p>
        </>
      )}

      {(status === 'cancelled' || status === 'expired' || result.isError) && (
        <>
          <p className="muted">
            {status === 'cancelled'
              ? 'Вхід скасовано.'
              : status === 'expired'
                ? 'Час на вхід минув.'
                : 'Не вдалося перевірити вхід.'}
          </p>
          <button type="button" className="button primary" onClick={restart}>
            Спробувати ще раз
          </button>
        </>
      )}
    </main>
  )
}
