import { m } from 'motion/react'
import { type FormEvent, useState } from 'react'
import { Link, useParams } from 'react-router'

import type { Schemas } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { formatShortDate } from '../shared/format'
import { CheckIcon } from '../shared/icons'
import { Screen } from '../shared/Screen'
import { successFeedback } from '../shared/haptics'
import { SPRING } from '../shared/motion'
import { useNoSwipeToClose } from './gestures'
import { finishWorkout, useOutboxStatus } from './outbox'
import { EFFORT_TEXT, progress, useMyWorkout } from './workouts'

const EFFORTS: Schemas['Effort'][] = ['easy', 'ok', 'hard']

/** The end of a workout: how it felt and a note for Dasha. */
export function FinishPage() {
  useNoSwipeToClose()
  const { id = '' } = useParams()
  const workout = useMyWorkout(id)
  const { waiting, failing } = useOutboxStatus()
  const [effort, setEffort] = useState<Schemas['Effort'] | null>(null)
  const [comment, setComment] = useState('')
  const [sent, setSent] = useState(false)
  // Sending without saying how it was asks for that first.
  const [asked, setAsked] = useState(false)
  const [openedAt] = useState(() => Date.now())
  const back = <BackLink to={`/app/workouts/${id}`} label="Тренування" />

  if (!workout.data) {
    return (
      <Screen>
        {back}
        <p className="muted">Завантаження…</p>
      </Screen>
    )
  }

  const { total, done, firstDoneAt } = progress(workout.data)
  // From the first ✓; a first ✓ from another day says nothing about this session.
  const minutes = firstDoneAt === null ? null : Math.round((openedAt - firstDoneAt) / 60_000)
  const sessionMinutes = minutes !== null && minutes <= 6 * 60 ? Math.max(minutes, 1) : null

  if (sent || workout.data.status === 'done') {
    const queued = waiting > 0 && failing
    // Just finished: a small moment. Opened again later, simply the result.
    const rise = (delay: number) =>
      sent
        ? {
            initial: { opacity: 0, y: 10 },
            animate: { opacity: 1, y: 0 },
            transition: { ...SPRING, delay },
          }
        : {}
    return (
      <Screen>
        <div className="finish-done">
          <m.span
            className={sent ? 'round-icon done big celebrate' : 'round-icon done big'}
            initial={sent ? { scale: 0 } : false}
            animate={{ scale: 1 }}
            transition={{ type: 'spring', stiffness: 380, damping: 13, delay: 0.05 }}
          >
            {sent && (
              <m.span
                className="finish-ring"
                aria-hidden="true"
                initial={{ scale: 1, opacity: 0.7 }}
                animate={{ scale: 2.4, opacity: 0 }}
                transition={{ duration: 0.9, ease: 'easeOut', delay: 0.2 }}
              />
            )}
            <CheckIcon size={28} />
          </m.span>
          <m.h1 {...rise(0.15)}>{queued ? 'Звіт збережено' : 'Звіт надіслано Даші'}</m.h1>
          <m.p className="muted" {...rise(0.22)}>
            {queued
              ? 'Немає зв’язку, тож звіт надішлеться сам, щойно телефон підключиться.'
              : 'Вона подивиться результати й напише, якщо щось треба змінити.'}
          </m.p>
          <m.div className="finish-action" {...rise(0.3)}>
            <Link className="button primary block" to="/app">
              На головну
            </Link>
          </m.div>
        </div>
      </Screen>
    )
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!effort) {
      setAsked(true)
      return
    }
    const minutes = firstDoneAt === null ? null : Math.round((Date.now() - firstDoneAt) / 60_000)
    finishWorkout(id, {
      effort,
      comment: comment.trim(),
      // A first ✓ from another day says nothing about this session's length.
      duration_min: minutes !== null && minutes <= 6 * 60 ? minutes : null,
    })
    successFeedback()
    setSent(true)
  }

  return (
    <Screen>
      {back}
      <header className="workout-head">
        <p className="muted small">
          {workout.data.title || 'Тренування'} · {formatShortDate(workout.data.date)}
        </p>
        <h1>Завершити тренування</h1>
      </header>

      {/* What was done leads; the questions come after. */}
      <ul className="stats">
        <li>
          <span className="muted small">Підходи</span>
          <strong>
            {done} <small>з {total}</small>
          </strong>
        </li>
        <li>
          <span className="muted small">Вправи</span>
          <strong>{workout.data.exercises.length}</strong>
        </li>
        {sessionMinutes !== null && (
          <li>
            <span className="muted small">Час</span>
            <strong>
              {sessionMinutes} <small>хв</small>
            </strong>
          </li>
        )}
      </ul>

      <form className="stack" onSubmit={submit}>
        <fieldset className="effort">
          <legend>Як було?</legend>
          <div className="effort-options">
            {EFFORTS.map((value) => (
              <button
                key={value}
                type="button"
                className="effort-option"
                aria-pressed={effort === value}
                onClick={() => {
                  setEffort(value)
                  setAsked(false)
                }}
              >
                {EFFORT_TEXT[value]}
              </button>
            ))}
          </div>
          {asked && <p className="error">Обери, як було.</p>}
        </fieldset>
        <label className="field">
          <span>Коментар для Даші</span>
          <textarea
            rows={4}
            maxLength={2000}
            placeholder="Що пішло важко, що боліло, що змінити — необов’язково"
            value={comment}
            onChange={(event) => setComment(event.target.value)}
          />
        </label>
        <button type="submit" className="button primary block">
          Надіслати звіт
        </button>
        {done < total && (
          <p className="muted small">
            Невиконані підходи Даша побачить як пропущені. Можна повернутися й позначити їх.
          </p>
        )}
      </form>
    </Screen>
  )
}
