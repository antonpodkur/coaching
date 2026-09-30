import { useEffect } from 'react'
import { Link, useNavigate } from 'react-router'

import { telegramWebApp } from '../app/telegram'
import { BackIcon } from './icons'

/**
 * Way back from a sub-page. Inside Telegram this is the back arrow in
 * Telegram's own header; in a browser it is a link at the top of the page.
 */
export function BackLink({ to, label }: { to: string; label: string }) {
  const navigate = useNavigate()
  const webApp = telegramWebApp()

  useEffect(() => {
    if (!webApp) return
    const back = () => void navigate(to)
    webApp.BackButton.onClick(back)
    webApp.BackButton.show()
    return () => {
      webApp.BackButton.offClick(back)
      webApp.BackButton.hide()
    }
  }, [webApp, navigate, to])

  if (webApp) return null
  return (
    <Link className="back-link" to={to}>
      <BackIcon />
      {label}
    </Link>
  )
}
