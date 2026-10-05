import type { Schemas } from '../api/client'

type Session = Schemas['MiniAppSession']

/** The Mini App's session in the query cache. */
export const SESSION_KEY = ['mini-app-session']

const STORAGE_KEY = 'mini_app_session'
/** A little under the token's 12 hours, so a cached session is never expired. */
const MAX_AGE_MS = 11 * 60 * 60 * 1000

interface Saved {
  session: Session
  telegramUserId: number | undefined
  savedAt: number
}

/** Keeps the session so the app can open without signal, e.g. in the gym. */
export function rememberSession(session: Session, telegramUserId: number | undefined) {
  try {
    const saved: Saved = { session, telegramUserId, savedAt: Date.now() }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(saved))
  } catch {
    // Without storage the app just needs a network to open.
  }
}

/** The remembered session, if it is recent and belongs to this Telegram user. */
export function recalledSession(telegramUserId: number | undefined): Session | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const saved = JSON.parse(raw) as Saved
    const fresh = Date.now() - saved.savedAt < MAX_AGE_MS
    return fresh && saved.telegramUserId === telegramUserId ? saved.session : null
  } catch {
    return null
  }
}
