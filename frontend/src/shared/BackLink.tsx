import { useEffect } from 'react'
import { Link, useNavigate } from 'react-router'

import { setSwipeBack } from '../app/gestures'
import { type TelegramWebApp, telegramWebApp } from '../app/telegram'
import { goBackTo } from './history'
import { BackIcon } from './icons'

/** Back links on the screen now; Telegram's back arrow shows while there is one. */
let backLinks = 0
let arrowShown = false

/**
 * Shows or hides Telegram's back arrow once the new screen is in. Going from
 * one screen with a back arrow to another, the old one's goes before the new
 * one's comes; told both, Telegram would animate the arrow away and back.
 */
function syncBackButton(webApp: TelegramWebApp) {
  queueMicrotask(() => {
    const show = backLinks > 0
    if (show === arrowShown) return
    arrowShown = show
    if (show) webApp.BackButton.show()
    else webApp.BackButton.hide()
  })
}

/**
 * Way back from a sub-page. Inside Telegram this is the back arrow in
 * Telegram's own header; elsewhere a link at the top of the page. On iPhones,
 * in Telegram or installed on the home screen, a swipe from the left edge too.
 */
export function BackLink({ to, label }: { to: string; label: string }) {
  const navigate = useNavigate()
  const webApp = telegramWebApp()

  useEffect(() => {
    const back = () => goBackTo(navigate, to)
    const releaseSwipe = setSwipeBack(back)
    if (!webApp) return releaseSwipe
    webApp.BackButton.onClick(back)
    backLinks += 1
    syncBackButton(webApp)
    return () => {
      webApp.BackButton.offClick(back)
      backLinks -= 1
      syncBackButton(webApp)
      releaseSwipe()
    }
  }, [webApp, navigate, to])

  if (webApp) return null
  return (
    <Link
      className="back-link"
      to={to}
      onClick={(event) => {
        event.preventDefault()
        goBackTo(navigate, to)
      }}
    >
      <BackIcon />
      {label}
    </Link>
  )
}
