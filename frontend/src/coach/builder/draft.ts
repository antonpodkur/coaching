import type { Schemas } from '../../api/client'
import {
  type Measure,
  formatDone,
  formatSecsInput,
  formatTarget,
  parseSecs,
} from '../../shared/format'

export type Workout = Schemas['Workout']

/** A set's target as the builder edits it. */
export interface DraftSet {
  id: string
  /** `null` is no weight; for bodyweight and timed exercises, kg is extra weight. */
  kg: number | null
  /** Reps, or seconds for a timed exercise. */
  reps_min: number
  reps_max: number
}

export interface DraftExercise {
  id: string
  exercise_id: string
  name: string
  measure: Measure
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
export const DEFAULT_SECS = 30

export function toDraft(workout: Workout): Draft {
  return {
    title: workout.title,
    date: workout.date ?? null,
    exercises: workout.exercises.map((exercise) => ({
      id: exercise.id,
      exercise_id: exercise.exercise_id,
      name: exercise.name,
      measure: exercise.measure,
      thumbnail_url: exercise.thumbnail_url ?? null,
      per_side_label: exercise.per_side_label ?? null,
      note: exercise.note ?? null,
      sets: exercise.sets.map((set) => ({
        id: set.id,
        kg: set.kg ?? null,
        reps_min: set.reps_min,
        reps_max: set.reps_max,
      })),
      last_time: exercise.last_time
        .map((set) => formatDone(set.kg, set.reps, exercise.measure))
        .join(' · '),
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

/** A new set repeats the one before it. */
export function newSet(measure: Measure, from?: DraftSet): DraftSet {
  const count = measure === 'time' ? DEFAULT_SECS : DEFAULT_REPS
  return {
    id: crypto.randomUUID(),
    kg: from?.kg ?? null,
    reps_min: from?.reps_min ?? count,
    reps_max: from?.reps_max ?? count,
  }
}

/** The count as typed: `8-10` or `12` reps, or `45`, `1:30`, `30-45` for time. */
export function formatCountInput(
  set: Pick<DraftSet, 'reps_min' | 'reps_max'>,
  measure: Measure,
): string {
  const one = (value: number) => (measure === 'time' ? formatSecsInput(value) : `${value}`)
  return set.reps_min === set.reps_max ? one(set.reps_min) : `${one(set.reps_min)}-${one(set.reps_max)}`
}

/** What a set chip says: `79 × 8–10`, `17 повт.`, `+10 × 8` or `45 с`. */
export function setLabel(set: DraftSet, measure: Measure): string {
  return formatTarget(set.kg, set.reps_min, set.reps_max, measure)
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

/** `45`, `1:30`, `10 хв` or a range `30-45` → seconds as min and max. */
export function parseTime(text: string): { reps_min: number; reps_max: number } | undefined {
  const parts = text.split(/[-–]/)
  if (parts.length > 2) return undefined
  const min = parseSecs(parts[0] ?? '')
  const max = parts[1] === undefined ? min : parseSecs(parts[1])
  if (!min || !max || max < min) return undefined
  return { reps_min: min, reps_max: max }
}

/** A step on the −/+ buttons for time: 5 s, then 15 s, then whole minutes. */
export function timeStep(secs: number): number {
  return secs < 60 ? 5 : secs < 300 ? 15 : 60
}

/** Arms by default; legs for leg and glute exercises. */
export function perSideLabel(muscleGroup: string | null | undefined): string {
  return muscleGroup === 'Ноги' || muscleGroup === 'Сідниці' ? 'на кожну ногу' : 'на кожну руку'
}
