import { useState } from 'react'

import type { Schemas } from '../api/client'
import { telegramWebApp } from '../app/telegram'
import { formatDay } from '../shared/format'
import { SendIcon } from '../shared/icons'

export type ShownInvite = Schemas['InviteLink'] & { name: string }

const SHARE_TEXT = 'Запрошення в застосунок тренувань Даші: відкрий посилання й натисни Start.'

/** Telegram's "share to a chat" screen with the invite filled in. */
function shareUrl(inviteUrl: string) {
  return `https://t.me/share/url?url=${encodeURIComponent(inviteUrl)}&text=${encodeURIComponent(SHARE_TEXT)}`
}

/** A fresh invite link, ready to send to the person in Telegram. */
export function InviteCard({ invite, onClose }: { invite: ShownInvite; onClose?: () => void }) {
  const [copied, setCopied] = useState(false)
  const webApp = telegramWebApp()
  const share = shareUrl(invite.url)

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
        <span className="tag tag-warn">до {formatDay(invite.expires_at)}</span>
      </div>
      <input
        className="invite-url"
        readOnly
        value={invite.url}
        aria-label="Посилання-запрошення"
        onFocus={(event) => event.target.select()}
      />
      {webApp ? (
        <button
          type="button"
          className="button primary block"
          onClick={() => webApp.openTelegramLink(share)}
        >
          <SendIcon />
          Надіслати в Telegram
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
        Telegram відкриє список чатів: обери людину й надішли. Посилання одноразове. Коли людина
        натисне Start, бот напише тобі.
      </p>
      {onClose && (
        <button type="button" className="link-button" onClick={onClose}>
          Закрити
        </button>
      )}
    </div>
  )
}
