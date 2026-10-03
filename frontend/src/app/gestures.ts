import { useEffect } from 'react'

import { telegramSupporting } from './telegram'

/**
 * Keeps a swipe down from closing the app in the middle of a workout. Scrolling
 * still works, and so does Telegram's own close button.
 */
export function useNoSwipeToClose() {
  useEffect(() => {
    const webApp = telegramSupporting('7.7')
    if (!webApp) return
    webApp.disableVerticalSwipes()
    return () => webApp.enableVerticalSwipes()
  }, [])
}
