import { useQuery } from '@tanstack/react-query'
import { useEffect, useMemo } from 'react'

import { type Schemas, api, unwrap } from '../api/client'
import { type OutboxEntry, settle, useOutbox } from './outbox'

export type ClientWorkout = Schemas['ClientWorkout']
export type ClientWorkoutSummary = Schemas['ClientWorkoutSummary']
export type ClientSet = Schemas['ClientSet']

export const MY_WORKOUTS_KEY = ['client-workouts']

/**
 * The last copy of each screen's data stays in localStorage, so a workout
 * opened at the gym door still shows with no signal inside.
 */
function cached<T>(key: string): T | undefined {
  try {
    const saved = localStorage.getItem(key)
    return saved ? (JSON.parse(saved) as T) : undefined
  } catch {
    return undefined
  }
}

function keep(key: string, data: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(data))
  } catch {
    // Full or blocked storage only costs the offline copy.
  }
}

export function useMyWorkouts() {
  const query = useQuery({
    queryKey: MY_WORKOUTS_KEY,
    queryFn: async () => {
      const workouts = unwrap(await api.GET('/me/workouts'))
      keep('cache:my-workouts', workouts)
      return workouts
    },
    initialData: () => cached<ClientWorkoutSummary[]>('cache:my-workouts'),
    // Shown at once, but refreshed straight away when there is a network.
    initialDataUpdatedAt: 0,
  })
  const outbox = useOutbox()
  const data = useMemo(
    () => query.data?.map((workout) => withQueuedSummary(workout, outbox)),
    [query.data, outbox],
  )
  // Once the list on screen shows a sent report, the phone's copy can go.
  useEffect(() => {
    if (!query.data) return
    const done = new Set(
      query.data.filter((workout) => workout.status === 'done').map((workout) => workout.id),
    )
    settle((entry) => entry.kind === 'finish' && done.has(entry.workoutId))
  }, [query.data])
  return { ...query, data }
}

export function useMyWorkout(id: string) {
  const cacheKey = `cache:workout:${id}`
  const query = useQuery({
    queryKey: [...MY_WORKOUTS_KEY, id],
    queryFn: async () => {
      const workout = unwrap(await api.GET('/workouts/{id}', { params: { path: { id } } }))
      keep(cacheKey, workout)
      return workout
    },
    initialData: () => cached<ClientWorkout>(cacheKey),
    initialDataUpdatedAt: 0,
  })
  const outbox = useOutbox()
  const data = useMemo(
    () => (query.data ? withQueued(query.data, outbox) : undefined),
    [query.data, outbox],
  )
  // Sent changes this copy shows are forgotten only now that it is on screen.
  useEffect(() => {
    const workout = query.data
    if (workout) settle((entry) => entry.workoutId === workout.id && shownIn(workout, entry))
  }, [query.data])
  return { ...query, data }
}

/** Whether the server's copy of the workout has this change, or a newer one. */
function shownIn(workout: ClientWorkout, entry: OutboxEntry): boolean {
  if (entry.kind === 'finish') return workout.status === 'done'
  const set = workout.exercises
    .flatMap((exercise) => exercise.sets)
    .find((candidate) => candidate.id === entry.setId)
  // Gone from the plan: nothing to show it on.
  if (!set) return true
  if (!set.client_updated_at) return false
  return Date.parse(set.client_updated_at) >= Date.parse(entry.result.client_updated_at)
}

/** The workout as the client sees it: the server's copy plus changes it does not show yet. */
function withQueued(workout: ClientWorkout, outbox: OutboxEntry[]): ClientWorkout {
  const queued = outbox.filter(
    (entry) => entry.workoutId === workout.id && !(entry.sentAt && shownIn(workout, entry)),
  )
  if (queued.length === 0) return workout
  const results = new Map<string, Schemas['SetResult']>()
  let finish: Schemas['FinishWorkout'] | undefined
  for (const entry of queued) {
    if (entry.kind === 'result') results.set(entry.setId, entry.result)
    else finish = entry.report
  }
  return {
    ...workout,
    status: finish ? 'done' : workout.status,
    report: finish
      ? {
          effort: finish.effort,
          comment: finish.comment ?? '',
          duration_min: finish.duration_min,
          finished_at: new Date().toISOString(),
        }
      : workout.report,
    exercises: workout.exercises.map((exercise) => ({
      ...exercise,
      sets: exercise.sets.map((set) => {
        const result = results.get(set.id)
        return result
          ? {
              ...set,
              actual_kg: result.actual_kg,
              actual_reps: result.actual_reps,
              completed: result.completed,
              client_updated_at: result.client_updated_at,
            }
          : set
      }),
    })),
  }
}

/** The list only needs "how many sets are done" and "finished" corrected. */
function withQueuedSummary(
  workout: ClientWorkoutSummary,
  outbox: OutboxEntry[],
): ClientWorkoutSummary {
  const queued = outbox.filter((entry) => entry.workoutId === workout.id)
  if (queued.length === 0) return workout
  const finished = queued.some((entry) => entry.kind === 'finish')
  return { ...workout, status: finished ? 'done' : workout.status }
}

/** Sets done and sets differing from the plan, for progress and the report. */
export function progress(workout: ClientWorkout) {
  let total = 0
  let done = 0
  let different = 0
  let firstDoneAt: number | null = null
  for (const exercise of workout.exercises) {
    for (const set of exercise.sets) {
      total += 1
      if (!set.completed) continue
      done += 1
      if (differs(set)) different += 1
      const at = set.client_updated_at ? Date.parse(set.client_updated_at) : NaN
      if (!Number.isNaN(at) && (firstDoneAt === null || at < firstDoneAt)) firstDoneAt = at
    }
  }
  return { total, done, different, firstDoneAt }
}

/** Logged numbers that are not what Dasha planned. */
export function differs(set: ClientSet): boolean {
  const kgDiffers = (set.actual_kg ?? null) !== (set.target_kg ?? null)
  const reps = set.actual_reps ?? null
  const repsDiffer = reps === null || reps < set.target_reps_min || reps > set.target_reps_max
  return kgDiffers || repsDiffer
}

export const EFFORT_TEXT: Record<Schemas['Effort'], string> = {
  easy: 'Легко',
  ok: 'Нормально',
  hard: 'Важко',
}

