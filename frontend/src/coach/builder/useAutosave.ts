import { useCallback, useEffect, useState } from 'react'

import { ApiError, api, unwrap } from '../../api/client'
import { holdClosing } from '../../shared/closingGuard'
import { type Draft, type Workout, toChanges, toDraft } from './draft'

export type SaveStatus = 'saved' | 'pending' | 'saving' | 'error' | 'conflict'

/** Wait for a pause in editing before saving. */
const DEBOUNCE_MS = 700
/** After a failed save (e.g. no signal), try again this soon. */
const RETRY_MS = 4_000

/**
 * Saves the builder's working copy as a whole, one request at a time. The
 * working copy is the truth while she edits; the server only hands back the
 * next `version`. A 409 means the workout changed elsewhere, and saving stops
 * until she reloads.
 */
class DraftSaver {
  latest: Draft
  private onServer: Draft
  private version: number
  private running: Promise<void> | null = null
  private timer: ReturnType<typeof setTimeout> | undefined
  stopped = false
  private readonly workoutId: string
  private readonly report: (status: SaveStatus) => void
  private readonly onUnauthorized: () => void

  constructor(
    workout: Workout,
    report: (status: SaveStatus) => void,
    onUnauthorized: () => void,
  ) {
    this.workoutId = workout.id
    this.report = report
    this.onUnauthorized = onUnauthorized
    this.latest = toDraft(workout)
    this.onServer = this.latest
    this.version = workout.version
  }

  get dirty() {
    return this.latest !== this.onServer
  }

  change(next: Draft) {
    this.latest = next
    this.report('pending')
    this.schedule(DEBOUNCE_MS)
  }

  async save(): Promise<void> {
    clearTimeout(this.timer)
    if (this.running) await this.running
    if (this.stopped || !this.dirty) return

    const sending = this.latest
    this.report('saving')
    this.running = this.send(sending)
    await this.running
    this.running = null

    // Edits made meanwhile, or a failed attempt: go again.
    if (!this.stopped && this.dirty) {
      this.schedule(this.latest === sending ? RETRY_MS : DEBOUNCE_MS)
    }
  }

  /** Saves now; `true` once the server has everything. */
  async flush(): Promise<boolean> {
    await this.save()
    return !this.stopped && !this.dirty
  }

  private schedule(delay: number) {
    clearTimeout(this.timer)
    this.timer = setTimeout(() => void this.save(), delay)
  }

  private async send(sending: Draft) {
    try {
      const saved = unwrap(
        await api.PUT('/coach/workouts/{id}', {
          params: { path: { id: this.workoutId }, header: { 'If-Match': String(this.version) } },
          body: toChanges(sending),
        }),
      )
      this.version = saved.version
      this.onServer = sending
      this.report(this.dirty ? 'pending' : 'saved')
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        this.stopped = true
        this.report('conflict')
      } else if (err instanceof ApiError && err.status === 401) {
        this.onUnauthorized()
      } else {
        this.report('error')
      }
    }
  }
}

/**
 * The builder's working copy, saved shortly after each change. `onUnauthorized`
 * is read once; both workspaces pass a stable callback.
 */
export function useAutosave(workout: Workout, onUnauthorized: () => void) {
  const [status, setStatus] = useState<SaveStatus>('saved')
  const [saver] = useState(() => new DraftSaver(workout, setStatus, onUnauthorized))
  const [draft, setDraft] = useState(saver.latest)

  const update = useCallback(
    (change: (current: Draft) => Draft) => {
      if (saver.stopped) return
      const next = change(saver.latest)
      saver.change(next)
      setDraft(next)
    },
    [saver],
  )
  const flush = useCallback(() => saver.flush(), [saver])

  // Telegram asks before closing while changes are unsaved.
  useEffect(() => {
    holdClosing('workout-draft', status !== 'saved' && status !== 'conflict')
  }, [status])

  // Leaving the screen saves what is left; the request finishes in the background.
  useEffect(
    () => () => {
      holdClosing('workout-draft', false)
      if (saver.dirty) void saver.save()
    },
    [saver],
  )

  return { draft, update, status, flush }
}
