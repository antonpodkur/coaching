import { useState } from 'react'

import { Collapse } from './Collapse'
import { enablePush, pushSupported } from './push'

const DISMISSED_KEY = 'push_card_dismissed'

function readDismissed(): boolean {
  try {
    return localStorage.getItem(DISMISSED_KEY) === '1'
  } catch {
    return false
  }
}

/**
 * Outside Telegram, offers notifications from the app itself, like any app
 * on the phone; a tap on one opens its screen. Shown until they are allowed
 * or refused, or put off with "Не зараз". The bot's messages keep coming too.
 *
 * Whether to show it is known when the page draws, so it never pops in late.
 * Allowed or put off, it folds away.
 */
export function PushCard({ forCoach = false }: { forCoach?: boolean }) {
  const [dismissed, setDismissed] = useState(readDismissed)
  const [status, setStatus] = useState<'ask' | 'working' | 'failed' | 'done'>('ask')

  const show =
    !dismissed &&
    status !== 'done' &&
    pushSupported() &&
    (status !== 'ask' || Notification.permission === 'default')

  const enable = async () => {
    setStatus('working')
    const result = await enablePush()
    setStatus(result === 'failed' ? 'failed' : 'done')
  }

  const dismiss = () => {
    try {
      localStorage.setItem(DISMISSED_KEY, '1')
    } catch {
      // Without storage it simply shows again next time.
    }
    setDismissed(true)
  }

  return (
    <Collapse show={show}>
      <div className="prompt-card">
        <strong>Сповіщення на телефоні</strong>
        <p className="muted small">
          {forCoach
            ? 'Звіти клієнтів, нові відео техніки й підсумок дня — одразу на екрані телефону.'
            : 'Нові тренування, нагадування в день тренування й норма харчування — одразу на екрані телефону.'}
        </p>
        <div className="prompt-actions">
          <button
            type="button"
            className="button primary small"
            disabled={status === 'working'}
            onClick={() => void enable()}
          >
            Увімкнути
          </button>
          <button type="button" className="link-button" onClick={dismiss}>
            Не зараз
          </button>
        </div>
        {status === 'failed' && <p className="error">Не вдалося увімкнути. Спробуй пізніше.</p>}
      </div>
    </Collapse>
  )
}
