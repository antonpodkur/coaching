import type { Schemas } from '../../api/client'
import { formatDone, formatTarget } from '../../shared/format'

export type Workout = Schemas['Workout']

/** A set's target as the builder edits it. */
export interface DraftSet {
  id: string
  /** `null` is bodyweight. */
  kg: number | null
  reps_min: number
  reps_max: number
}

export interface DraftExercise {
  id: string
  exercise_id: string
  name: string
  thumbnail_url: string | null
  per_side_label: string | null
  note: string | null
  sets: DraftSet[]
  /** What the client did last time, e.g. `34 × 12 · 34 × 9`. Read-only. */
  last_time: string
}

/** The workout the builder holds; saved to the server as a whole. */
export interface Draft {
  title: string
  date: string | null
  exercises: DraftExercise[]
}

/** A fresh exercise starts with one set; she sets the numbers right away. */
export const DEFAULT_REPS = 10

export function toDraft(workout: Workout): Draft {
  return {
    title: workout.title,
    date: workout.date ?? null,
    exercises: workout.exercises.map((exercise) => ({
      id: exercise.id,
      exercise_id: exercise.exercise_id,
      name: exercise.name,
      thumbnail_url: exercise.thumbnail_url ?? null,
      per_side_label: exercise.per_side_label ?? null,
      note: exercise.note ?? null,
      sets: exercise.sets.map((set) => ({
        id: set.id,
        kg: set.kg ?? null,
        reps_min: set.reps_min,
        reps_max: set.reps_max,
      })),
      last_time: exercise.last_time.map((set) => formatDone(set.kg, set.reps)).join(' · '),
    })),
  }
}

export function toChanges(draft: Draft): Schemas['WorkoutChanges'] {
  return {
    title: draft.title,
    date: draft.date,
    exercises: draft.exercises.map((exercise) => ({
      id: exercise.id,
      exercise_id: exercise.exercise_id,
      per_side_label: exercise.per_side_label,
      note: exercise.note,
      sets: exercise.sets,
    })),
  }
}

export function newSet(from?: DraftSet): DraftSet {
  return {
    id: crypto.randomUUID(),
    kg: from?.kg ?? null,
    reps_min: from?.reps_min ?? DEFAULT_REPS,
    reps_max: from?.reps_max ?? DEFAULT_REPS,
  }
}

/** `8-10` or `12`. */
export function formatReps(set: Pick<DraftSet, 'reps_min' | 'reps_max'>): string {
  return set.reps_min === set.reps_max ? `${set.reps_min}` : `${set.reps_min}-${set.reps_max}`
}

/** What a set chip says: `79 × 8–10`, or `17 повт.` for bodyweight. */
export function setLabel(set: DraftSet): string {
  return formatTarget(set.kg, set.reps_min, set.reps_max)
}

/** `79`, `27,5` or `` for bodyweight → a number, `null`, or `undefined` if unreadable. */
export function parseKg(text: string): number | null | undefined {
  const trimmed = text.trim().replace(',', '.')
  if (trimmed === '') return null
  const kg = Number(trimmed)
  return Number.isFinite(kg) && kg > 0 && kg <= 999 ? Math.round(kg * 100) / 100 : undefined
}

/** `12` or `8-10` → min and max, or `undefined` if unreadable. */
export function parseReps(text: string): { reps_min: number; reps_max: number } | undefined {
  const match = text.trim().match(/^(\d{1,3})(?:\s*[-–]\s*(\d{1,3}))?$/)
  if (!match) return undefined
  const min = Number(match[1])
  const max = match[2] ? Number(match[2]) : min
  return min >= 1 && max >= min && max <= 500 ? { reps_min: min, reps_max: max } : undefined
}

/** Arms by default; legs for leg and glute exercises. */
export function perSideLabel(muscleGroup: string | null | undefined): string {
  return muscleGroup === 'Ноги' || muscleGroup === 'Сідниці' ? 'на кожну ногу' : 'на кожну руку'
}
