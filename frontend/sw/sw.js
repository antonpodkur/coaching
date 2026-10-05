// @ts-check
/// <reference lib="webworker" />

/**
 * The service worker keeps this version's own files on the phone, so the app
 * opens with no signal. The build (sw/plugin.ts) fills in `BUILD`. The API,
 * videos and photos live at other addresses and always go to the network;
 * workouts and logged sets are kept by the app itself.
 *
 * A new version downloads in the background and waits. The app offers
 * "Оновити", which sends 'activate' (src/shared/appUpdate.ts); otherwise it
 * takes over once every window of the app has closed.
 *
 * It also shows the app's notifications (web push, see backend/src/push.rs):
 * a tap opens the notification's screen, and the app's icon shows how many
 * are waiting.
 */

const worker = /** @type {ServiceWorkerGlobalScope} */ (/** @type {unknown} */ (self))

/** @type {{ version: string, files: string[] }} */
const BUILD = { version: 'dev', files: [] }
const CACHE = `app-${BUILD.version}`
/** Every page is the same app, `index.html`; the router picks the screen. */
const PAGE = '/'

worker.addEventListener('install', (event) => {
  event.waitUntil(keepFiles())
})

worker.addEventListener('activate', (event) => {
  event.waitUntil(dropOldVersions().then(() => worker.clients.claim()))
})

worker.addEventListener('message', (event) => {
  if (event.data === 'activate') void worker.skipWaiting()
})

worker.addEventListener('fetch', (event) => {
  const request = event.request
  const url = new URL(request.url)
  if (request.method !== 'GET' || url.origin !== worker.location.origin) return
  // A file opened directly, such as /sw.js, is not a page of the app.
  if (request.mode === 'navigate' && !/\.\w+$/.test(url.pathname)) {
    event.respondWith(fromCache(PAGE, request))
  } else if (BUILD.files.includes(url.pathname)) {
    event.respondWith(fromCache(url.pathname, request))
  }
})

worker.addEventListener('push', (event) => {
  event.waitUntil(showPush(event.data))
})

worker.addEventListener('notificationclick', (event) => {
  event.notification.close()
  const url = event.notification.data?.url
  event.waitUntil(openScreen(typeof url === 'string' ? url : '/app').then(showCount))
})

/**
 * What the server sent: `{ title, body, url, tag }` (backend/src/push.rs).
 * @param {PushMessageData | null} data
 * @returns {{ title?: string, body?: string, url?: string, tag?: string }}
 */
function readPush(data) {
  try {
    return data?.json() ?? {}
  } catch {
    return { body: data?.text() }
  }
}

/** @param {PushMessageData | null} data */
async function showPush(data) {
  const message = readPush(data)
  await worker.registration.showNotification(message.title ?? 'Тренування', {
    body: message.body,
    tag: message.tag,
    icon: '/icon-192.png',
    data: { url: message.url ?? '/app' },
  })
  await showCount()
}

/**
 * The app's window goes to the screen if it is open (the page routes it
 * without reloading); otherwise the app opens on it.
 * @param {string} url
 */
async function openScreen(url) {
  const windows = await worker.clients.matchAll({ type: 'window', includeUncontrolled: true })
  const open = windows[0]
  if (open) {
    open.postMessage({ type: 'open', url })
    try {
      await open.focus()
      return
    } catch {
      // Some phones refuse to bring it forward; open it instead.
    }
  }
  await worker.clients.openWindow(url)
}

/** The number on the app's icon: notifications still waiting on the phone. */
async function showCount() {
  const waiting = await worker.registration.getNotifications()
  const badge = worker.navigator
  if (waiting.length > 0) await badge.setAppBadge?.(waiting.length).catch(() => undefined)
  else await badge.clearAppBadge?.().catch(() => undefined)
}

async function keepFiles() {
  const cache = await caches.open(CACHE)
  await Promise.all(
    BUILD.files.map(async (file) => {
      // Hashed files never change, so the previous version's copy will do.
      const kept = file.startsWith('/assets/') ? await caches.match(file) : undefined
      const response = kept ?? (await fetch(file, { cache: 'no-cache' }))
      if (!response.ok) throw new Error(`${file}: ${response.status}`)
      // A page must not be answered with a redirect it never asked for.
      await cache.put(file, response.redirected ? new Response(response.body, response) : response)
    }),
  )
}

async function dropOldVersions() {
  const names = await caches.keys()
  await Promise.all(names.filter((name) => name !== CACHE).map((name) => caches.delete(name)))
}

/**
 * The kept copy, or the network if the phone dropped it.
 * @param {string} file
 * @param {Request} request
 */
async function fromCache(file, request) {
  const kept = await caches.match(file, { cacheName: CACHE })
  return kept ?? fetch(request)
}
