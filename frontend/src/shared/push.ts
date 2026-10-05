import { api, unwrap } from '../api/client'
import { telegramWebApp } from '../app/telegram'
import { installedApp, phoneKind } from './device'

/**
 * Whether this browser can get notifications from the app itself (web push):
 * outside Telegram, in a built app (its service worker shows them), and on
 * an iPhone only once the app is installed. Inside Telegram the bot writes.
 */
export function pushSupported(): boolean {
  return (
    import.meta.env.PROD &&
    !telegramWebApp() &&
    'serviceWorker' in navigator &&
    'PushManager' in window &&
    'Notification' in window &&
    (phoneKind() !== 'iphone' || installedApp())
  )
}

function keyBytes(key: string): Uint8Array<ArrayBuffer> {
  const base64 = key.replace(/-/g, '+').replace(/_/g, '/')
  return Uint8Array.from(atob(base64), (char) => char.charCodeAt(0))
}

function sameKey(current: ArrayBuffer | null, key: Uint8Array) {
  if (!current) return false
  const bytes = new Uint8Array(current)
  return bytes.length === key.length && bytes.every((byte, index) => byte === key[index])
}

/** Subscribes this phone, or keeps its subscription, and gives it to whoever is signed in. */
async function subscribe() {
  const { public_key } = unwrap(await api.GET('/push/key'))
  const key = keyBytes(public_key)
  const registration = await navigator.serviceWorker.ready
  let subscription = await registration.pushManager.getSubscription()
  // Made with an older server key, it would no longer get anything.
  if (subscription && !sameKey(subscription.options.applicationServerKey, key)) {
    await subscription.unsubscribe()
    subscription = null
  }
  subscription ??= await registration.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: key,
  })
  const json = subscription.toJSON()
  unwrap(
    await api.PUT('/push/subscription', {
      body: {
        endpoint: subscription.endpoint,
        keys: { p256dh: json.keys?.p256dh ?? '', auth: json.keys?.auth ?? '' },
      },
    }),
  )
}

/**
 * Asks the phone to allow notifications, then subscribes. Call it straight
 * from a tap: an iPhone only asks in answer to one.
 */
export async function enablePush(): Promise<'enabled' | 'denied' | 'failed'> {
  const permission = await Notification.requestPermission()
  if (permission !== 'granted') return 'denied'
  try {
    await subscribe()
    return 'enabled'
  } catch {
    return 'failed'
  }
}

/**
 * Each time the app opens: where notifications are allowed, this phone's
 * subscription belongs to whoever is signed in now, and stays current.
 */
export async function syncPush() {
  if (!pushSupported() || Notification.permission !== 'granted') return
  try {
    await subscribe()
  } catch {
    // The next open tries again.
  }
}

/** Signing out: this phone stops getting their notifications. `token` is still theirs. */
export async function stopPush(token: string) {
  if (!pushSupported()) return
  try {
    const registration = await navigator.serviceWorker.ready
    const subscription = await registration.pushManager.getSubscription()
    if (!subscription) return
    await api.DELETE('/push/subscription', {
      body: { endpoint: subscription.endpoint },
      headers: { Authorization: `Bearer ${token}` },
    })
    await subscription.unsubscribe()
  } catch {
    // The server forgets it once the push service reports it gone.
  }
}

/**
 * A tap on a notification while the app is open goes to its screen (the
 * service worker asks); the number on the app's icon clears while it is open.
 */
export function listenForNotifications(open: (url: string) => void) {
  if (!('serviceWorker' in navigator)) return
  navigator.serviceWorker.addEventListener('message', (event: MessageEvent) => {
    const data: unknown = event.data
    if (
      typeof data === 'object' &&
      data !== null &&
      'type' in data &&
      data.type === 'open' &&
      'url' in data &&
      typeof data.url === 'string' &&
      data.url.startsWith('/app')
    ) {
      open(data.url)
    }
  })
  const clearBadge = () => {
    if (document.visibilityState === 'visible') navigator.clearAppBadge?.().catch(() => undefined)
  }
  clearBadge()
  document.addEventListener('visibilitychange', clearBadge)
}
