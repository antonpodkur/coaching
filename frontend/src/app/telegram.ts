/**
 * The parts of Telegram's Mini App SDK (loaded in index.html) that the app uses.
 * Reference: https://core.telegram.org/bots/webapps
 */
export interface TelegramWebApp {
  /** Signed launch data; the backend verifies it. Empty outside Telegram. */
  initData: string
  initDataUnsafe: { start_param?: string; user?: { id: number } }
  ready(): void
  /** Opens a t.me link inside Telegram; the Mini App stays open. */
  openTelegramLink(url: string): void
  /** Makes Telegram ask before the app is closed, e.g. during an upload. */
  enableClosingConfirmation(): void
  disableClosingConfirmation(): void
  /** A native OK/Cancel dialog. */
  showConfirm(message: string, callback: (confirmed: boolean) => void): void
  /** The back arrow in Telegram's own header. */
  BackButton: {
    show(): void
    hide(): void
    onClick(callback: () => void): void
    offClick(callback: () => void): void
  }
  expand(): void
  setHeaderColor(color: string): void
  setBackgroundColor(color: string): void
  HapticFeedback: {
    impactOccurred(style: 'light' | 'medium' | 'heavy' | 'rigid' | 'soft'): void
    notificationOccurred(type: 'error' | 'success' | 'warning'): void
  }
}

declare global {
  interface Window {
    Telegram?: { WebApp: TelegramWebApp }
  }
}

/** The SDK when the page runs inside Telegram, `null` in a normal browser. */
export function telegramWebApp(): TelegramWebApp | null {
  const webApp = window.Telegram?.WebApp
  return webApp?.initData ? webApp : null
}
