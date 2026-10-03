import { useQueryClient } from '@tanstack/react-query'
import { type ChangeEvent, useState } from 'react'
import { Link, useParams } from 'react-router'

import { type Schemas, api } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { FormVideoTile } from '../shared/FormVideoTile'
import { VideoPlayer } from '../shared/VideoPlayer'
import {
  type Measure,
  type TimeUnit,
  formatDone,
  formatInUnit,
  formatKg,
  formatTarget,
  parseInUnit,
  plural,
  timeUnitFor,
} from '../shared/format'
import { CameraIcon, CheckIcon, ChevronIcon, CloseIcon } from '../shared/icons'
import { Screen } from '../shared/Screen'
import { useNoSwipeToClose } from './gestures'
import { logSet } from './outbox'
import { telegramWebApp } from './telegram'
import { dismissSend, sendVideo, useVideoSends } from './videoSends'
import { type ClientSet, MY_WORKOUTS_KEY, differs, useMyWorkout } from './workouts'

/**
 * One exercise in the gym: Dasha's video and note, last time's numbers, and
 * the sets. ✓ means "done as planned"; if it went differently, change the
 * numbers first. Every change is kept on the phone and sent when possible.
 */
export function ExerciseScreen() {
  useNoSwipeToClose()
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
  // Times are typed in the unit the plan reads best in: `60` for 60 хв.
  const unit = timeUnitFor(...exercise.sets.flatMap((set) => [set.target_reps_min, set.target_reps_max]))
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

      {exercise.note && (
        <div className="notice pink coach-note">
          <span>Коментар від Даші</span>
          <p>{exercise.note}</p>
        </div>
      )}
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
          <span>{measure === 'time' ? UNIT_HEAD[unit] : 'Повт.'}</span>
          <span />
        </div>
        {exercise.sets.map((set, setIndex) => (
          <SetRow
            key={set.id}
            workoutId={id}
            set={set}
            number={setIndex + 1}
            measure={measure}
            unit={unit}
            showKg={showKg}
          />
        ))}
      </section>

      <p className="muted small">
        {done === 0
          ? '✓ — зроблено як у плані. Було інакше — зміни цифри, а тоді натисни ✓.'
          : `Виконано ${done} з ${exercise.sets.length}.${different > 0 ? ` Інакше, ніж у плані: ${different} — Даша побачить це у звіті.` : ''}`}
      </p>

      <FormVideos
        workoutId={id}
        exercise={exercise}
        enabled={workout.data.videos_enabled === true}
      />

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

/** Videos the client sends Dasha of how they did this exercise. */
const MAX_VIDEOS = 3

function FormVideos({
  workoutId,
  exercise,
  enabled,
}: {
  workoutId: string
  exercise: { id: string; name: string; videos?: Schemas['FormVideo'][] }
  enabled: boolean
}) {
  const queryClient = useQueryClient()
  const sends = useVideoSends().filter((send) => send.workoutExerciseId === exercise.id)
  // Workouts cached before videos existed have no list.
  const videos = exercise.videos ?? []
  if (!enabled && videos.length === 0) return null

  const counted =
    videos.filter((video) => video.status !== 'failed').length +
    sends.filter((send) => send.phase !== 'failed').length

  const pick = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) void sendVideo(workoutId, exercise.id, file, queryClient)
  }
  const remove = (videoId: string) => {
    const deleteIt = (confirmed: boolean) => {
      if (!confirmed) return
      void api
        .DELETE('/form-videos/{id}', { params: { path: { id: videoId } } })
        .finally(
          () => void queryClient.invalidateQueries({ queryKey: [...MY_WORKOUTS_KEY, workoutId] }),
        )
    }
    const question = 'Видалити це відео? Даша його більше не побачить.'
    const webApp = telegramWebApp()
    if (webApp) webApp.showConfirm(question, deleteIt)
    else deleteIt(window.confirm(question))
  }

  return (
    <section className="form-videos" aria-labelledby={`videos-${exercise.id}`}>
      <h2 id={`videos-${exercise.id}`} className="section-title">
        Відео для Даші
      </h2>
      {videos.map((video, index) => (
        <FormVideoTile
          key={video.id}
          video={video}
          label={`Відео ${index + 1}`}
          onDelete={() => remove(video.id)}
        />
      ))}
      {sends.map((send) => (
        <div key={send.key} className={send.phase === 'failed' ? 'video-send failed' : 'video-send'}>
          <div className="video-send-head">
            <span className="small">
              {send.phase === 'failed'
                ? send.error
                : send.phase === 'uploading'
                  ? `Надсилаю відео… ${Math.round(send.progress * 100)}%`
                  : 'Готую відео…'}
            </span>
            {send.phase === 'failed' && (
              <button
                type="button"
                className="icon-button"
                aria-label="Закрити"
                onClick={() => dismissSend(send.key)}
              >
                <CloseIcon />
              </button>
            )}
          </div>
          {send.phase !== 'failed' && (
            <div
              className="progress"
              role="progressbar"
              aria-valuenow={Math.round(send.progress * 100)}
              aria-valuemin={0}
              aria-valuemax={100}
            >
              <div style={{ width: `${Math.round(send.progress * 100)}%` }} />
            </div>
          )}
        </div>
      ))}
      {enabled && counted < MAX_VIDEOS && (
        <label className="button block file-button">
          <CameraIcon />
          Надіслати відео Даші
          <input type="file" accept="video/*" onChange={pick} />
        </label>
      )}
      {enabled && (
        <p className="muted small">
          Зніми підхід збоку, щоб було видно все тіло. До 3 хвилин і до 3 відео на вправу. Відео
          бачить лише Даша.
        </p>
      )}
    </section>
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

const UNIT_HEAD: Record<TimeUnit, string> = { sec: 'Сек', min: 'Хв', hour: 'Год' }
const UNIT_WORD: Record<TimeUnit, string> = { sec: 'секунди', min: 'хвилини', hour: 'години' }

interface SetRowProps {
  workoutId: string
  set: ClientSet
  number: number
  measure: Measure
  /** What a time is typed in. */
  unit: TimeUnit
  /** Without it the set has no weight at all. */
  showKg: boolean
}

function SetRow({ workoutId, set, number, measure, unit, showKg }: SetRowProps) {
  const timed = measure === 'time'
  // Untouched sets start from the plan; the top of a range is the aim.
  const startKg = set.completed || set.client_updated_at ? set.actual_kg : set.target_kg
  const startReps = set.completed || set.client_updated_at ? set.actual_reps : set.target_reps_max
  const [kgText, setKgText] = useState(startKg == null ? '' : formatKg(startKg))
  const [repsText, setRepsText] = useState(
    startReps == null ? '' : timed ? formatInUnit(startReps, unit) : String(startReps),
  )
  const readCount = timed ? (text: string) => parseInUnit(text, unit) : readReps
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
        // Decimal, so `1,5` minutes can be typed.
        inputMode={timed ? 'decimal' : 'numeric'}
        aria-label={timed ? `Підхід ${number}: час, ${UNIT_WORD[unit]}` : `Підхід ${number}: повтори`}
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
