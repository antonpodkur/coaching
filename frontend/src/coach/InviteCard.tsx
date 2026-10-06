import { useState } from 'react'

import type { Schemas } from '../api/client'
import { telegramSupporting, telegramWebApp } from '../app/telegram'
import { onPhone } from '../shared/device'
import { formatDay } from '../shared/format'
import { SendIcon } from '../shared/icons'
import { useCoach } from './context'

export type ShownInvite = Schemas['InviteLink'] & { name: string }

/** What goes with the link. The name stays as written: Ukrainian would change it by case. */
function shareText(coachName: string) {
  return `${coachName} запрошує тебе на онлайн-тренування. Відкрий посилання — застосунок відкриється в Telegram.`
}

/** Telegram's "share to a chat" screen with the invite filled in. */
function shareUrl(inviteUrl: string, text: string) {
  return `https://t.me/share/url?url=${encodeURIComponent(inviteUrl)}&text=${encodeURIComponent(text)}`
}

/**
 * A fresh invite, ready to send in Telegram. Inside Telegram it goes as a card
 * with an "Відкрити" button that opens the app; elsewhere as a plain link,
 * through the phone's share sheet or Telegram's share page.
 */
export function InviteCard({ invite, onClose }: { invite: ShownInvite; onClose?: () => void }) {
  const [copied, setCopied] = useState(false)
  const [sent, setSent] = useState(false)
  const webApp = telegramWebApp()
  const text = shareText(useCoach().coach.name)
  const share = shareUrl(invite.url, text)
  const cardSharer = invite.prepared_message_id ? telegramSupporting('8.0') : null
  const preparedId = invite.prepared_message_id
  const shareSheet = !webApp && onPhone() && typeof navigator.share === 'function'

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(invite.url)
      setCopied(true)
    } catch {
      setCopied(false)
    }
  }

  return (
    <div className="invite-card" role="status">
      <div className="invite-head">
        <strong>Запрошення · {invite.name}</strong>
        <span className="tag">до {formatDay(invite.expires_at)}</span>
      </div>
      <input
        className="invite-url"
        readOnly
        value={invite.url}
        aria-label="Посилання-запрошення"
        onFocus={(event) => event.target.select()}
      />
      {cardSharer && preparedId ? (
        <button
          type="button"
          className="button primary block"
          onClick={() => cardSharer.shareMessage(preparedId, (done) => setSent(done))}
        >
          <SendIcon />
          {sent ? 'Надіслано. Надіслати ще раз' : 'Надіслати запрошення'}
        </button>
      ) : webApp ? (
        <button
          type="button"
          className="button primary block"
          onClick={() => webApp.openTelegramLink(share)}
        >
          <SendIcon />
          Надіслати в Telegram
        </button>
      ) : shareSheet ? (
        <button
          type="button"
          className="button primary block"
          onClick={() =>
            void navigator.share({ text, url: invite.url }).catch(() => undefined)
          }
        >
          <SendIcon />
          Надіслати запрошення
        </button>
      ) : (
        <a className="button primary block" href={share} target="_blank" rel="noreferrer">
          <SendIcon />
          Надіслати в Telegram
        </a>
      )}
      <button type="button" className="button block" onClick={copy}>
        {copied ? 'Скопійовано' : 'Скопіювати посилання'}
      </button>
      <p className="muted small">
        Обери людину в списку чатів. Вона натисне «Відкрити» й одразу потрапить у застосунок, а бот
        напише тобі. Посилання одноразове.
      </p>
      {onClose && (
        <button type="button" className="link-button" onClick={onClose}>
          Закрити
        </button>
      )}
    </div>
  )
}
