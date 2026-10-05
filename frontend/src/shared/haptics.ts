import { telegramWebApp } from '../app/telegram'

/**
 * A light tap under the finger, e.g. for a ticked set: Telegram's haptics
 * inside Telegram, a short vibration on Android elsewhere. iPhone browsers
 * have neither.
 */
export function tapFeedback(style: 'light' | 'soft') {
  const webApp = telegramWebApp()
  if (webApp) webApp.HapticFeedback.impactOccurred(style)
  else navigator.vibrate?.(10)
}

/** Something finished well, e.g. a sent report. */
export function successFeedback() {
  const webApp = telegramWebApp()
  if (webApp) webApp.HapticFeedback.notificationOccurred('success')
  else navigator.vibrate?.([15, 80, 15])
}
