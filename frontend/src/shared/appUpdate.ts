import { useSyncExternalStore } from 'react'

/** An app left open for days still asks for a new version this often. */
const CHECK_EVERY_MS = 30 * 60 * 1000

/** A new version, downloaded and waiting for "Оновити". */
let waiting: ServiceWorker | null = null
const listeners = new Set<() => void>()

function setWaiting(worker: ServiceWorker) {
  waiting = worker
  for (const listener of listeners) listener()
}

/**
 * Registers the service worker (sw/sw.js) in a built app, so it opens with
 * no signal. Where service workers don't run, e.g. inside Telegram on an
 * iPhone, the app works as before.
 */
export function registerServiceWorker() {
  if (!import.meta.env.PROD || !('serviceWorker' in navigator)) return
  window.addEventListener('load', () => void register())
}

async function register() {
  let registration: ServiceWorkerRegistration
  try {
    registration = await navigator.serviceWorker.register('/sw.js')
  } catch {
    return
  }

  // Without a controller this is the first install, not a new version.
  const offer = (worker: ServiceWorker | null) => {
    if (worker && navigator.serviceWorker.controller) setWaiting(worker)
  }
  offer(registration.waiting)
  registration.addEventListener('updatefound', () => {
    const worker = registration.installing
    worker?.addEventListener('statechange', () => {
      if (worker.state === 'installed') offer(worker)
    })
  })

  // The phone checks when the app opens; one that stays open checks on return.
  let checkedAt = Date.now()
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState !== 'visible' || Date.now() - checkedAt < CHECK_EVERY_MS) return
    checkedAt = Date.now()
    registration.update().catch(() => undefined)
  })
}

/** Switches to the waiting version and reloads on the same screen. */
export function applyUpdate() {
  if (!waiting) return
  navigator.serviceWorker.addEventListener('controllerchange', () => window.location.reload(), {
    once: true,
  })
  waiting.postMessage('activate')
}

/** Whether a new version is waiting. */
export function useUpdateReady(): boolean {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
    () => waiting !== null,
  )
}
