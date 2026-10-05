import { useMutation, useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'

import { api, unwrap } from '../api/client'
import { onPhone } from '../shared/device'
import type { AppSession } from './session'

const POLL_INTERVAL_MS = 2000

/**
 * Sign-in outside Telegram, confirmed in the bot: open Telegram, press Start
 * and Confirm, and this screen collects the session. Dasha gets her
 * workspace, a client who joined gets their workouts.
 */
export function SignIn({ onSignedIn }: { onSignedIn: (session: AppSession) => void }) {
  const start = useMutation({
    mutationFn: async () => unwrap(await api.POST('/auth/bot-login')),
  })
  const login = start.data
  // Ready as the screen opens, so "Відкрити Telegram" is the only tap here.
  const { mutate } = start
  useEffect(() => mutate(), [mutate])

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
    // Confirming means switching to Telegram, which hides this screen (on a
    // phone the app may be suspended entirely): keep polling while hidden, and
    // check again the moment they come back.
    refetchIntervalInBackground: true,
    refetchOnWindowFocus: true,
    retry: false,
    gcTime: 0,
  })

  const status = result.data?.status
  useEffect(() => {
    if (result.data?.status === 'approved') onSignedIn(result.data.session)
  }, [result.data, onSignedIn])

  const restart = () => {
    start.reset()
    start.mutate()
  }

  const failed = start.isError || result.isError
  const waiting = !failed && (status === undefined || status === 'pending')
  const problem =
    status === 'refused'
      ? 'Цей Telegram-акаунт ще не має доступу. Спершу відкрий у Telegram посилання-запрошення від тренера, а потім увійди тут.'
      : status === 'cancelled'
        ? 'Вхід скасовано.'
        : status === 'expired'
          ? 'Час на вхід минув.'
          : start.isError
            ? 'Не вдалося почати вхід. Перевір зв’язок.'
            : result.isError
              ? 'Не вдалося перевірити вхід.'
              : null

  return (
    <main className="sign-in">
      <h1>Вхід</h1>
      <p className="muted">Вхід підтверджується в Telegram, у чаті з ботом.</p>

      {waiting && (
        <>
          <p className="login-code" aria-live="polite">
            {login?.display_code ?? '····'}
          </p>
          {login ? (
            // A phone opens the Telegram app straight away; a computer gets t.me.
            onPhone() ? (
              <a className="button primary" href={login.bot_app_url}>
                Відкрити Telegram
              </a>
            ) : (
              <a className="button primary" href={login.bot_url} target="_blank" rel="noreferrer">
                Відкрити Telegram
              </a>
            )
          ) : (
            <button type="button" className="button primary" disabled>
              Відкрити Telegram
            </button>
          )}
          <p className="muted login-hint">
            У Telegram натисни «Почати» й підтверди вхід — там буде той самий код. Потім повернись
            сюди.
          </p>
        </>
      )}

      {problem && (
        <>
          <p className="muted login-hint">{problem}</p>
          <button type="button" className="button primary" onClick={restart}>
            Спробувати ще раз
          </button>
        </>
      )}
    </main>
  )
}
