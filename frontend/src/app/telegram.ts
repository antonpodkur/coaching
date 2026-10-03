/**
 * The parts of Telegram's Mini App SDK (loaded in index.html) that the app uses.
 * Reference: https://core.telegram.org/bots/webapps
 */
export type HomeScreenStatus = 'unsupported' | 'unknown' | 'added' | 'missed'

export interface TelegramWebApp {
  /** Signed launch data; the backend verifies it. Empty outside Telegram. */
  initData: string
  initDataUnsafe: {
    /** From an app link `t.me/<bot>?startapp=<param>`, e.g. an invite. */
    start_param?: string
    user?: { id: number; allows_write_to_pm?: boolean }
  }
  /** Whether the user's Telegram supports this Bot API version or newer. */
  isVersionAtLeast(version: string): boolean
  ready(): void
  /** Opens a t.me link inside Telegram; the Mini App stays open. */
  openTelegramLink(url: string): void
  /** Makes Telegram ask before the app is closed, e.g. during an upload. */
  enableClosingConfirmation(): void
  disableClosingConfirmation(): void
  /** A native OK/Cancel dialog. */
  showConfirm(message: string, callback: (confirmed: boolean) => void): void
  showAlert(message: string, callback?: () => void): void
  /** Telegram's own "allow the bot to message you?" popup (Bot API 6.9). */
  requestWriteAccess(callback: (granted: boolean) => void): void
  /** Shares a message card the bot prepared (Bot API 8.0). */
  shareMessage(messageId: string, callback?: (sent: boolean) => void): void
  /** Offers an icon on the phone's home screen (Bot API 8.0). */
  addToHomeScreen(): void
  checkHomeScreenStatus(callback: (status: HomeScreenStatus) => void): void
  onEvent(event: 'homeScreenAdded', handler: () => void): void
  offEvent(event: 'homeScreenAdded', handler: () => void): void
  /** Swipe down to close or minimize the app (Bot API 7.7). */
  disableVerticalSwipes(): void
  enableVerticalSwipes(): void
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

/** The SDK if this Telegram supports `version` of the Bot API, else `null`. */
export function telegramSupporting(version: string): TelegramWebApp | null {
  const webApp = telegramWebApp()
  return webApp?.isVersionAtLeast(version) ? webApp : null
}
