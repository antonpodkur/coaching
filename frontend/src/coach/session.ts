const STORAGE_KEY = 'coach_token'

// Storage can throw in private windows; the coach then signs in each visit.

export function loadCoachToken(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY)
  } catch {
    return null
  }
}

export function saveCoachToken(token: string | null) {
  try {
    if (token) localStorage.setItem(STORAGE_KEY, token)
    else localStorage.removeItem(STORAGE_KEY)
  } catch {
    // Not persisted; the session still works until the tab closes.
  }
}
