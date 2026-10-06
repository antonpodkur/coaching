import { Link, useParams } from 'react-router'

import { BackLink } from '../shared/BackLink'
import { formatShortDate, formatTarget, plural } from '../shared/format'
import { CheckIcon } from '../shared/icons'
import { Screen } from '../shared/Screen'
import { useNoSwipeToClose } from './gestures'
import { EFFORT_TEXT, progress, useMyWorkout } from './workouts'

/** One workout: Dasha's exercises in order, with progress, and the way to finish. */
export function WorkoutPage() {
  useNoSwipeToClose()
  const { id = '' } = useParams()
  const workout = useMyWorkout(id)
  const back = <BackLink to="/app" label="Головна" />

  if (!workout.data) {
    return (
      <Screen>
        {back}
        <p className="muted">
          {workout.isError
            ? 'Не вдалося відкрити тренування.'
            : workout.fetchStatus === 'paused'
              ? 'Немає зв’язку. Тренування відкриється, щойно телефон підключиться.'
              : 'Завантаження…'}
        </p>
      </Screen>
    )
  }

  const data = workout.data
  const { total, done } = progress(data)
  // "Start" or "continue" goes to the first exercise with sets left.
  const next =
    data.exercises.find((exercise) => exercise.sets.some((set) => !set.completed)) ??
    data.exercises[0]
  const finished = data.status === 'done'

  return (
    <Screen>
      {back}
      <header className="workout-head">
        <p className="muted small">{formatShortDate(data.date)} · від Даші</p>
        <h1>{data.title || 'Тренування'}</h1>
        <p className="muted small">
          {data.exercises.length} {plural(data.exercises.length, 'вправа', 'вправи', 'вправ')} ·{' '}
          {total} {plural(total, 'підхід', 'підходи', 'підходів')}
          {done > 0 && ` · виконано ${done}`}
        </p>
        {done > 0 && (
          <div className="progress" role="progressbar" aria-label="Виконано підходів" aria-valuemin={0} aria-valuemax={total} aria-valuenow={done}>
            <div style={{ width: `${(done / Math.max(total, 1)) * 100}%` }} />
          </div>
        )}
      </header>

      {finished && data.report && (
        <section className="report-card" aria-label="Звіт">
          <strong>Звіт надіслано Даші</strong>
          <span className="muted small">
            {EFFORT_TEXT[data.report.effort]}
            {data.report.duration_min != null && ` · ${data.report.duration_min} хв`}
          </span>
          {data.report.comment && <p>«{data.report.comment}»</p>}
        </section>
      )}

      <ol className="client-exercises">
        {data.exercises.map((exercise, index) => {
          const doneSets = exercise.sets.filter((set) => set.completed).length
          const complete = exercise.sets.length > 0 && doneSets === exercise.sets.length
          return (
            <li key={exercise.id}>
              <Link className="client-exercise" to={`/app/workouts/${id}/exercises/${exercise.id}`}>
                {exercise.video ? (
                  <img className="thumb" src={exercise.video.thumbnail_url} alt="" loading="lazy" />
                ) : (
                  // No video: its place in the workout instead of an empty frame.
                  <span className="thumb thumb-number" aria-hidden="true">
                    {index + 1}
                  </span>
                )}
                <span className="exercise-text">
                  <span className="exercise-name">{exercise.name}</span>
                  <span className="exercise-meta">
                    {exercise.sets
                      .map((set) =>
                        formatTarget(
                          set.target_kg,
                          set.target_reps_min,
                          set.target_reps_max,
                          exercise.measure,
                        ),
                      )
                      .join(' · ')}
                    {exercise.per_side_label && ` · ${exercise.per_side_label}`}
                  </span>
                  {exercise.note && <span className="exercise-note">{exercise.note}</span>}
                </span>
                <span className={complete ? 'set-count complete' : 'set-count'}>
                  {complete ? <CheckIcon /> : `${doneSets}/${exercise.sets.length}`}
                </span>
              </Link>
            </li>
          )
        })}
      </ol>

      {!finished && next && (
        <div className="stack">
          {/* Every set ticked: finishing is the step left, so it is the main button. */}
          {total > 0 && done === total ? (
            <Link className="button primary block" to={`/app/workouts/${id}/finish`}>
              Завершити й надіслати звіт
            </Link>
          ) : (
            <>
              <Link
                className="button primary block"
                to={`/app/workouts/${id}/exercises/${next.id}`}
              >
                {done > 0 ? 'Продовжити' : 'Почати тренування'}
              </Link>
              <Link className="button block" to={`/app/workouts/${id}/finish`}>
                Завершити й надіслати звіт
              </Link>
            </>
          )}
        </div>
      )}
    </Screen>
  )
}
