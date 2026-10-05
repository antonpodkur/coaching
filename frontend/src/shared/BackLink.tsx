import { useEffect } from 'react'
import { Link, useNavigate } from 'react-router'

import { setSwipeBack } from '../app/gestures'
import { telegramWebApp } from '../app/telegram'
import { BackIcon } from './icons'
import { goingBack } from './screenMotion'

/**
 * Way back from a sub-page. Inside Telegram this is the back arrow in
 * Telegram's own header; elsewhere a link at the top of the page. On iPhones,
 * in Telegram or installed on the home screen, a swipe from the left edge too.
 */
export function BackLink({ to, label }: { to: string; label: string }) {
  const navigate = useNavigate()
  const webApp = telegramWebApp()

  useEffect(() => {
    const back = () => {
      goingBack()
      void navigate(to)
    }
    const releaseSwipe = setSwipeBack(back)
    if (!webApp) return releaseSwipe
    webApp.BackButton.onClick(back)
    webApp.BackButton.show()
    return () => {
      webApp.BackButton.offClick(back)
      webApp.BackButton.hide()
      releaseSwipe()
    }
  }, [webApp, navigate, to])

  if (webApp) return null
  return (
    <Link className="back-link" to={to} onClick={goingBack}>
      <BackIcon />
      {label}
    </Link>
  )
}
