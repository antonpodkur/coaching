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
