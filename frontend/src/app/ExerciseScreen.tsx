import { useState } from 'react'
import { Link, useParams } from 'react-router'

import { BackLink } from '../shared/BackLink'
import { VideoPlayer } from '../shared/VideoPlayer'
import {
  type Measure,
  formatDone,
  formatKg,
  formatSecsInput,
  formatTarget,
  parseSecs,
  plural,
} from '../shared/format'
import { CheckIcon, ChevronIcon } from '../shared/icons'
import { Screen } from '../shared/Screen'
import { logSet } from './outbox'
import { telegramWebApp } from './telegram'
import { type ClientSet, differs, useMyWorkout } from './workouts'

/**
 * One exercise in the gym: Dasha's video and note, last time's numbers, and
 * the sets. ✓ means "done as planned"; if it went differently, change the
 * numbers first. Every change is kept on the phone and sent when possible.
 */
export function ExerciseScreen() {
  const { id = '', exerciseId = '' } = useParams()
  const workout = useMyWorkout(id)
  const back = <BackLink to={`/app/workouts/${id}`} label="Тренування" />

  const exercises = workout.data?.exercises ?? []
  const index = exercises.findIndex((exercise) => exercise.id === exerciseId)
  const exercise = exercises[index]
  if (!workout.data || !exercise) {
    return (
      <Screen>
        {back}
        <p className="muted">{workout.data ? 'Такої вправи немає.' : 'Завантаження…'}</p>
      </Screen>
    )
  }

  const next = exercises[index + 1]
  // Workouts cached on the phone before this field existed count as weight.
  const measure: Measure = exercise.measure ?? 'weight'
  // Bodyweight and timed exercises have a kg column only for extra weight.
  const showKg =
    measure === 'weight' ||
    exercise.sets.some((set) => set.target_kg != null || set.actual_kg != null)
  const done = exercise.sets.filter((set) => set.completed).length
  const different = exercise.sets.filter((set) => set.completed && differs(set)).length

  return (
    <Screen>
      {back}
      {exercise.video && (
        <VideoPlayer
          src={exercise.video.hls_url}
          poster={exercise.video.thumbnail_url}
          label={`Техніка від Даші: ${exercise.name}`}
        />
      )}

      <header className="exercise-head">
        <p className="muted small">
          Вправа {index + 1} з {exercises.length}
        </p>
        <h1>{exercise.name}</h1>
        <p className="muted small">
          {exercise.sets.length} {plural(exercise.sets.length, 'підхід', 'підходи', 'підходів')}
          {exercise.per_side_label && ` · ${exercise.per_side_label}`}
        </p>
      </header>

      {exercise.note && <p className="notice pink">{exercise.note}</p>}
      {exercise.last_time.length > 0 && (
        <p className="last-time">
          Минулого разу:{' '}
          <strong>
            {exercise.last_time.map((set) => formatDone(set.kg, set.reps, measure)).join(' · ')}
          </strong>
        </p>
      )}

      <section className={showKg ? 'set-table' : 'set-table no-kg'} aria-label="Підходи">
        <div className="set-table-head" aria-hidden="true">
          <span>№</span>
          <span>План</span>
          {showKg && <span>{measure === 'weight' ? 'Кг' : '+кг'}</span>}
          <span>{measure === 'time' ? 'Час' : 'Повт.'}</span>
          <span />
        </div>
        {exercise.sets.map((set, setIndex) => (
          <SetRow
            key={set.id}
            workoutId={id}
            set={set}
            number={setIndex + 1}
            measure={measure}
            showKg={showKg}
          />
        ))}
      </section>

      <p className="muted small">
        {done === 0
          ? '✓ — зроблено як у плані. Було інакше — зміни цифри, а тоді натисни ✓.'
          : `Виконано ${done} з ${exercise.sets.length}.${different > 0 ? ` Інакше, ніж у плані: ${different} — Даша побачить це у звіті.` : ''}`}
      </p>

      <div className="exercise-nav">
        <Link className="button" to={`/app/workouts/${id}`}>
          Усі вправи
        </Link>
        {next ? (
          <Link className="button primary" to={`/app/workouts/${id}/exercises/${next.id}`}>
            Наступна
            <ChevronIcon />
          </Link>
        ) : (
          <Link className="button primary" to={`/app/workouts/${id}/finish`}>
            Завершити
          </Link>
        )}
      </div>
    </Screen>
  )
}

/** Plain numbers, so `27,5` and `27.5` both work; empty kg means no weight. */
function readKg(text: string): number | null | undefined {
  const value = text.trim().replace(',', '.')
  if (value === '') return null
  const kg = Number(value)
  return Number.isFinite(kg) && kg > 0 && kg <= 999 ? kg : undefined
}

function readReps(text: string): number | null | undefined {
  const value = text.trim()
  if (value === '') return null
  const reps = Number(value)
  return Number.isInteger(reps) && reps >= 0 && reps <= 500 ? reps : undefined
}

interface SetRowProps {
  workoutId: string
  set: ClientSet
  number: number
  measure: Measure
  /** Without it the set has no weight at all. */
  showKg: boolean
}

function SetRow({ workoutId, set, number, measure, showKg }: SetRowProps) {
  const timed = measure === 'time'
  // Untouched sets start from the plan; the top of a range is the aim.
  const startKg = set.completed || set.client_updated_at ? set.actual_kg : set.target_kg
  const startReps = set.completed || set.client_updated_at ? set.actual_reps : set.target_reps_max
  const [kgText, setKgText] = useState(startKg == null ? '' : formatKg(startKg))
  const [repsText, setRepsText] = useState(
    startReps == null ? '' : timed ? formatSecsInput(startReps) : String(startReps),
  )
  const readCount = timed ? parseSecs : readReps
  const kg = readKg(kgText)
  const reps = readCount(repsText)
  const valid = kg !== undefined && reps !== undefined

  const send = (completed: boolean, nextKg = kg, nextReps = reps) => {
    if (nextKg === undefined || nextReps === undefined) return
    logSet(workoutId, set.id, {
      actual_kg: nextKg,
      actual_reps: nextReps,
      completed,
      client_updated_at: new Date().toISOString(),
    })
  }

  const toggle = () => {
    if (!valid) return
    telegramWebApp()?.HapticFeedback.impactOccurred(set.completed ? 'soft' : 'light')
    send(!set.completed)
  }

  // After ✓, corrections are logged straight away.
  const editKg = (text: string) => {
    setKgText(text)
    if (set.completed) send(true, readKg(text), reps)
  }
  const editReps = (text: string) => {
    setRepsText(text)
    if (set.completed) send(true, kg, readCount(text))
  }

  const kgDiffers = kg !== undefined && (kg ?? null) !== (set.target_kg ?? null)
  const repsDiffer =
    reps !== undefined && reps !== null && (reps < set.target_reps_min || reps > set.target_reps_max)
  const target = formatTarget(set.target_kg, set.target_reps_min, set.target_reps_max, measure)

  return (
    <div className={set.completed ? 'set-row completed' : 'set-row'}>
      <span className="set-number">{number}</span>
      <span className="set-target">{target}</span>
      {showKg && (
        <input
          className={kg === undefined ? 'bad' : kgDiffers ? 'differs' : undefined}
          inputMode="decimal"
          aria-label={`Підхід ${number}: ${measure === 'weight' ? 'вага' : 'додаткова вага'}, кг`}
          placeholder="—"
          value={kgText}
          onChange={(event) => editKg(event.target.value)}
        />
      )}
      <input
        className={reps === undefined ? 'bad' : repsDiffer ? 'differs' : undefined}
        // `1:30` needs the colon, which number pads lack.
        inputMode={timed && set.target_reps_max >= 60 ? 'text' : 'numeric'}
        aria-label={timed ? `Підхід ${number}: час, секунди або 1:30` : `Підхід ${number}: повтори`}
        value={repsText}
        onChange={(event) => editReps(event.target.value)}
      />
      <button
        type="button"
        className="set-check"
        aria-pressed={set.completed}
        aria-label={set.completed ? `Підхід ${number} виконано` : `Підхід ${number}: зроблено`}
        disabled={!valid}
        onClick={toggle}
      >
        <CheckIcon size={18} />
      </button>
    </div>
  )
}
