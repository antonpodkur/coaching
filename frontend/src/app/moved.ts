import { installedApp } from '../shared/device'

/** The app's first address, before it had a domain of its own. */
const OLD_HOST = 'coaching.anton-podkur.workers.dev'
/** The app's address since October 2026. */
export const APP_ORIGIN = 'https://getcoachin.app'

/**
 * On the old address, sends Telegram and browser tabs on to the same page at
 * the new one. Telegram's sign-in travels in the hash, so the Mini App opens
 * as before. An app installed from the old address can't follow (the phone
 * tied it to that address), so it stays for `MovedPage` to explain.
 */
export function leaveOldAddress(): 'here' | 'leaving' | 'installed' {
  if (window.location.host !== OLD_HOST) return 'here'
  if (installedApp()) return 'installed'
  const { pathname, search, hash } = window.location
  window.location.replace(APP_ORIGIN + pathname + search + hash)
  return 'leaving'
}
