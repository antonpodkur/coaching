import { useState } from 'react'
import { Navigate } from 'react-router'

import { Screen } from '../shared/Screen'
import { installedApp, otherBrowserOnIphone, phoneKind } from '../shared/device'
import { AddBoxIcon, CopyIcon, MenuDotsIcon, ShareIcon } from '../shared/icons'
import { type InstallPrompt, useInstallPrompt } from '../shared/installPrompt'

/**
 * How to put the app on the phone's home screen, for the phone at hand. The
 * Telegram version's "Встановити" opens this page in the phone's browser.
 */
export function InstallPage() {
  const [kind] = useState(phoneKind)

  // An installed app can open on this address if the phone kept it.
  if (installedApp()) return <Navigate to="/app" replace />

  return (
    <Screen>
      <header className="install-head">
        <img className="install-icon" src="/apple-touch-icon.png" alt="" width={64} height={64} />
        <h1>Застосунок на телефон</h1>
        <p className="muted">
          Іконка на екрані телефону: тренування відкриваються одним дотиком, без пошуку чату, і
          працюють навіть без зв’язку в залі.
        </p>
      </header>
      {kind === 'iphone' ? (
        <IphoneSteps />
      ) : kind === 'android' ? (
        <AndroidSteps />
      ) : (
        <ComputerSteps />
      )}
    </Screen>
  )
}

function IphoneSteps() {
  return (
    <>
      {otherBrowserOnIphone() && (
        <div className="notice install-note">
          <p>Відкрий цю сторінку в Safari: з нього застосунок додається найпростіше.</p>
          <CopyLink />
        </div>
      )}
      <ol className="install-steps">
        <li>
          <ShareIcon />
          <span>
            Натисни значок «Поділитися» (Share): квадрат зі стрілкою вгору. Не видно — спершу ⋯
            праворуч унизу.
          </span>
        </li>
        <li>
          <AddBoxIcon />
          <span>Прокрути меню й обери «На початковий екран» (Add to Home Screen).</span>
        </li>
        <li>
          <span className="install-step-mark">✓</span>
          <span>Натисни «Додати» (Add) — на екрані з’явиться іконка.</span>
        </li>
        <li>
          <span className="install-step-mark">→</span>
          <span>Відкрий застосунок з іконки й увійди через Telegram.</span>
        </li>
      </ol>
      <p className="muted small">
        Сторінка відкрилася всередині Telegram? Натисни ⋯ або значок компаса й обери «Відкрити в
        Safari».
      </p>
    </>
  )
}

function AndroidSteps() {
  const [installed, setInstalled] = useState(false)
  if (installed) {
    return (
      <p className="notice">Готово. Відкрий застосунок з іконки на екрані й увійди через Telegram.</p>
    )
  }
  return (
    <>
      <InstallButton label="Встановити" onInstalled={() => setInstalled(true)} />
      <ol className="install-steps">
        <li>
          <MenuDotsIcon />
          <span>У Chrome натисни ⋮ угорі праворуч.</span>
        </li>
        <li>
          <AddBoxIcon />
          <span>
            Обери «Встановити застосунок» або «Додати на головний екран» (Install app / Add to Home
            screen).
          </span>
        </li>
        <li>
          <span className="install-step-mark">→</span>
          <span>Відкрий застосунок з іконки й увійди через Telegram.</span>
        </li>
      </ol>
      <p className="muted small">
        Сторінка відкрилася всередині Telegram? Натисни ⋮ і обери «Відкрити в браузері».
      </p>
    </>
  )
}

function ComputerSteps() {
  return (
    <>
      <div className="install-note">
        <p>Відкрий цю сторінку на телефоні:</p>
        <CopyLink />
      </div>
      <InstallButton label="Встановити на цей комп’ютер" onInstalled={() => undefined} />
    </>
  )
}

/** Chrome's own install dialog, where Chrome offers one. */
function InstallButton({ label, onInstalled }: { label: string; onInstalled: () => void }) {
  const offered = useInstallPrompt()
  // Chrome's offer can be shown once.
  const [used, setUsed] = useState<InstallPrompt | null>(null)
  if (!offered || offered === used) return null

  const install = async () => {
    setUsed(offered)
    await offered.prompt()
    const { outcome } = await offered.userChoice
    if (outcome === 'accepted') onInstalled()
  }
  return (
    <button type="button" className="button primary block" onClick={() => void install()}>
      {label}
    </button>
  )
}

function CopyLink() {
  const [copied, setCopied] = useState(false)
  const url = window.location.href
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
    } catch {
      setCopied(false)
    }
  }
  return (
    <button type="button" className="button small" onClick={() => void copy()}>
      <CopyIcon />
      {copied ? 'Скопійовано' : 'Скопіювати посилання'}
    </button>
  )
}
