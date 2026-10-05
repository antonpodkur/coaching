import { useSyncExternalStore } from 'react'

import { ApiError, type Schemas, api, unwrap } from '../api/client'

/**
 * Changes the client made that the screens' copy of their workouts may not
 * show yet: logged sets and finished workouts. Gyms often have no signal, so
 * every change lands here first (and in localStorage, in case Telegram closes
 * the app), the screens show it at once, and it is sent in order whenever the
 * network allows. Sending twice is harmless: the server keeps the newest
 * change per set.
 *
 * A change the server took stays, marked `sent`, until a fresh copy of the
 * workout shows it (`settle`). Dropped any sooner, the screen would fall back
 * to the copy from before the change for a moment: a ticked set blinking off.
 */
export type OutboxEntry = (
  | {
      kind: 'result'
      key: string
      workoutId: string
      setId: string
      result: Schemas['SetResult']
    }
  | {
      kind: 'finish'
      key: string
      workoutId: string
      report: Schemas['FinishWorkout']
    }
) & {
  /** When the server took it; it is then only kept for the screens. */
  sentAt?: number
}

const STORAGE_KEY = 'outbox_v1'
/** Waits after a failed send; the last one repeats. */
const RETRY_MS = [2_000, 5_000, 15_000, 30_000, 60_000]
/** Sent changes no fresh copy has shown by then are forgotten when the app starts. */
const SENT_KEPT_MS = 2 * 24 * 60 * 60 * 1000

let entries: OutboxEntry[] = load()
/** Whether the last send failed, i.e. changes are waiting for a network. */
let failing = false
let snapshot = { entries, failing }
const listeners = new Set<() => void>()
const drainedListeners = new Set<() => void>()
let flushing = false
let failures = 0
let retryTimer: ReturnType<typeof setTimeout> | undefined

function load(): OutboxEntry[] {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    const loaded = saved ? (JSON.parse(saved) as OutboxEntry[]) : []
    return loaded.filter((entry) => !entry.sentAt || Date.now() - entry.sentAt < SENT_KEPT_MS)
  } catch {
    return []
  }
}

function unsent(): OutboxEntry[] {
  return entries.filter((entry) => !entry.sentAt)
}

function save() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(entries))
  } catch {
    // Storage is full or blocked; the entries still live in memory.
  }
  publish()
}

function publish() {
  snapshot = { entries, failing }
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** Every change the screens should show, sent or not. */
export function useOutbox(): OutboxEntry[] {
  return useSyncExternalStore(subscribe, () => snapshot).entries
}

/** Changes waiting to be sent, and whether sending them is failing right now. */
export function useOutboxStatus() {
  const { entries: all, failing: stuck } = useSyncExternalStore(subscribe, () => snapshot)
  return { waiting: all.filter((entry) => !entry.sentAt).length, failing: stuck }
}

/** Calls `listener` whenever the last waiting change has reached the server. */
export function onDrained(listener: () => void) {
  drainedListeners.add(listener)
  return () => {
    drainedListeners.delete(listener)
  }
}

/** Adds a change, replacing an older one for the same set or workout, and sends. */
export function enqueue(entry: OutboxEntry) {
  entries = [...entries.filter((queued) => queued.key !== entry.key), entry]
  save()
  void flush()
}

export function logSet(workoutId: string, setId: string, result: Schemas['SetResult']) {
  enqueue({ kind: 'result', key: `set:${setId}`, workoutId, setId, result })
}

export function finishWorkout(workoutId: string, report: Schemas['FinishWorkout']) {
  enqueue({ kind: 'finish', key: `finish:${workoutId}`, workoutId, report })
}

/**
 * Forgets sent changes the screens' fresh copy of a workout already shows.
 * Call it once that copy is on screen, so nothing falls back in between.
 */
export function settle(shown: (entry: OutboxEntry) => boolean) {
  const kept = entries.filter((entry) => !entry.sentAt || !shown(entry))
  if (kept.length === entries.length) return
  entries = kept
  save()
}

/** Sends waiting changes, oldest first, until done or the network fails. */
export async function flush() {
  if (flushing) return
  flushing = true
  clearTimeout(retryTimer)
  try {
    for (;;) {
      const entry = unsent()[0]
      if (!entry) break
      let refused = false
      try {
        await send(entry)
      } catch (err) {
        if (!retryable(err)) {
          console.error('dropping a change the server refused', entry, err)
          refused = true
        } else {
          failures += 1
          failing = true
          publish()
          const delay = RETRY_MS[Math.min(failures, RETRY_MS.length) - 1]
          retryTimer = setTimeout(() => void flush(), delay)
          return
        }
      }
      failures = 0
      failing = false
      // A newer change for the same set may have replaced this one meanwhile.
      entries = refused
        ? entries.filter((queued) => queued !== entry)
        : entries.map((queued) => (queued === entry ? { ...entry, sentAt: Date.now() } : queued))
      save()
      if (unsent().length === 0) for (const listener of drainedListeners) listener()
    }
  } finally {
    flushing = false
  }
}

async function send(entry: OutboxEntry) {
  if (entry.kind === 'result') {
    unwrap(
      await api.PUT('/sets/{id}/result', {
        params: { path: { id: entry.setId } },
        body: entry.result,
      }),
    )
  } else {
    unwrap(
      await api.POST('/workouts/{id}/finish', {
        params: { path: { id: entry.workoutId } },
        body: entry.report,
      }),
    )
  }
}

/** No network, a server hiccup, or an expired session: keep it and try later. */
function retryable(err: unknown): boolean {
  if (!(err instanceof ApiError)) return true
  return err.status >= 500 || err.status === 401 || err.status === 408 || err.status === 429
}

// Try again as soon as the phone is back online or the app comes to the front.
if (typeof window !== 'undefined') {
  window.addEventListener('online', () => void flush())
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') void flush()
  })
}
