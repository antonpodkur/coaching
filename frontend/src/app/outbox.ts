import { useSyncExternalStore } from 'react'

import { ApiError, type Schemas, api, unwrap } from '../api/client'

/**
 * Changes the client made that the server has not confirmed yet: logged sets
 * and finished workouts. Gyms often have no signal, so every change lands here
 * first (and in localStorage, in case Telegram closes the app), the screens
 * show it at once, and it is sent in order whenever the network allows.
 * Sending twice is harmless: the server keeps the newest change per set.
 */
export type OutboxEntry =
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

const STORAGE_KEY = 'outbox_v1'
/** Waits after a failed send; the last one repeats. */
const RETRY_MS = [2_000, 5_000, 15_000, 30_000, 60_000]

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
    return saved ? (JSON.parse(saved) as OutboxEntry[]) : []
  } catch {
    return []
  }
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

export function useOutbox(): OutboxEntry[] {
  return useSyncExternalStore(subscribe, () => snapshot).entries
}

/** Changes waiting, and whether sending them is failing right now. */
export function useOutboxStatus() {
  const { entries: waiting, failing: stuck } = useSyncExternalStore(subscribe, () => snapshot)
  return { waiting: waiting.length, failing: stuck }
}

/** Calls `listener` whenever the last queued change has reached the server. */
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

/** Sends queued changes, oldest first, until done or the network fails. */
export async function flush() {
  if (flushing) return
  flushing = true
  clearTimeout(retryTimer)
  try {
    while (entries.length > 0) {
      const entry = entries[0]
      if (!entry) break
      try {
        await send(entry)
      } catch (err) {
        if (!retryable(err)) {
          console.error('dropping a change the server refused', entry, err)
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
      entries = entries.filter((queued) => queued !== entry)
      save()
      if (entries.length === 0) for (const listener of drainedListeners) listener()
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
