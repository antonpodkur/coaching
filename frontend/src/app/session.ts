import { type Schemas, api } from '../api/client'

export type AppSession = Schemas['AppSession']

/** The signed-in person in the query cache, inside Telegram or not. */
export const SESSION_KEY = ['session']

// Inside Telegram every launch signs in afresh; the last session is kept for
// a gym with no signal.

const MINI_APP_KEY = 'mini_app_session'
/** A little under the token's 12 hours, so a cached session is never expired. */
const MAX_AGE_MS = 11 * 60 * 60 * 1000

interface Remembered {
  session: AppSession
  telegramUserId: number | undefined
  savedAt: number
}

/** Keeps the Mini App's session so it can open without signal, e.g. in the gym. */
export function rememberSession(session: AppSession, telegramUserId: number | undefined) {
  try {
    const saved: Remembered = { session, telegramUserId, savedAt: Date.now() }
    localStorage.setItem(MINI_APP_KEY, JSON.stringify(saved))
  } catch {
    // Without storage the app just needs a network to open.
  }
}

/** The remembered Mini App session, if it is recent and belongs to this Telegram user. */
export function recalledSession(telegramUserId: number | undefined): AppSession | null {
  try {
    const raw = localStorage.getItem(MINI_APP_KEY)
    if (!raw) return null
    const saved = JSON.parse(raw) as Remembered
    const fresh = Date.now() - saved.savedAt < MAX_AGE_MS
    return fresh && saved.telegramUserId === telegramUserId ? saved.session : null
  } catch {
    return null
  }
}

// Outside Telegram the session from the bot sign-in lasts a month and is
// renewed each time the app opens.

const SAVED_KEY = 'app_session'
/** Dasha's browser token from before everyone signed in this way. */
const OLD_COACH_KEY = 'coach_token'

/** The session saved on this phone or computer. */
export function savedSession(): AppSession | null {
  try {
    const raw = localStorage.getItem(SAVED_KEY)
    return raw ? (JSON.parse(raw) as AppSession) : null
  } catch {
    return null
  }
}

/** Saves the session, or forgets it with `null` (signing out). */
export function saveSession(session: AppSession | null) {
  try {
    if (session) localStorage.setItem(SAVED_KEY, JSON.stringify(session))
    else localStorage.removeItem(SAVED_KEY)
    localStorage.removeItem(OLD_COACH_KEY)
  } catch {
    // Not kept: the next visit asks to sign in again.
  }
}

/** Dasha's older browser token, which renews into a full session. */
export function oldCoachToken(): string | null {
  try {
    return localStorage.getItem(OLD_COACH_KEY)
  } catch {
    return null
  }
}

/**
 * Bot messages follow the phone's clock: the client's reminders, Dasha's
 * evening summary. Best effort; the next sign-in tries again.
 */
export async function syncTimezone(session: AppSession) {
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone
  if (!timezone) return
  if (session.role === 'client' && session.client.timezone !== timezone) {
    await api.PUT('/me/timezone', { body: { timezone } }).catch(() => undefined)
  }
  if (session.role === 'coach' && session.coach.timezone !== timezone) {
    await api.PUT('/coach/me/timezone', { body: { timezone } }).catch(() => undefined)
  }
}
