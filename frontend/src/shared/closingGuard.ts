import { telegramWebApp } from '../app/telegram'

/**
 * Reasons to ask before Telegram closes the Mini App: an upload running,
 * changes not saved yet. Telegram asks while any reason holds.
 */
const reasons = new Set<string>()

export function holdClosing(reason: string, hold: boolean) {
  if (hold) reasons.add(reason)
  else reasons.delete(reason)
  const webApp = telegramWebApp()
  if (reasons.size > 0) webApp?.enableClosingConfirmation()
  else webApp?.disableClosingConfirmation()
}
