import { telegramWebApp } from '../app/telegram'

/** Telegram's own confirm dialog inside the Mini App, the browser's elsewhere. */
export function confirmAction(message: string): Promise<boolean> {
  const webApp = telegramWebApp()
  if (!webApp) return Promise.resolve(window.confirm(message))
  return new Promise((resolve) => webApp.showConfirm(message, resolve))
}

/** Telegram's own alert inside the Mini App, the browser's elsewhere. */
export function alertMessage(message: string) {
  const webApp = telegramWebApp()
  if (webApp) webApp.showAlert(message)
  else window.alert(message)
}
