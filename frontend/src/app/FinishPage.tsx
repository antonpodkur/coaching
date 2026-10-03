import { type FormEvent, useState } from 'react'
import { Link, useParams } from 'react-router'

import type { Schemas } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { plural } from '../shared/format'
import { CheckIcon } from '../shared/icons'
import { Screen } from '../shared/Screen'
import { useNoSwipeToClose } from './gestures'
import { finishWorkout, useOutboxStatus } from './outbox'
import { telegramWebApp } from './telegram'
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
  const back = <BackLink to={`/app/workouts/${id}`} label="Тренування" />

  if (!workout.data) {
    return (
      <Screen>
        {back}
        <p className="muted">Завантаження…</p>
      </Screen>
    )
  }

  const { total, done, different, firstDoneAt } = progress(workout.data)

  if (sent || workout.data.status === 'done') {
    const queued = waiting > 0 && failing
    return (
      <Screen>
        <div className="finish-done">
          <span className="round-icon done big">
            <CheckIcon size={28} />
          </span>
          <h1>{queued ? 'Звіт збережено' : 'Звіт надіслано Даші'}</h1>
          <p className="muted">
            {queued
              ? 'Немає зв’язку, тож звіт надішлеться сам, щойно телефон підключиться.'
              : 'Вона подивиться результати й напише, якщо щось треба змінити.'}
          </p>
          <Link className="button primary block" to="/app">
            На головну
          </Link>
        </div>
      </Screen>
    )
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!effort) return
    const minutes = firstDoneAt === null ? null : Math.round((Date.now() - firstDoneAt) / 60_000)
    finishWorkout(id, {
      effort,
      comment: comment.trim(),
      // A first ✓ from another day says nothing about this session's length.
      duration_min: minutes !== null && minutes <= 6 * 60 ? minutes : null,
    })
    telegramWebApp()?.HapticFeedback.notificationOccurred('success')
    setSent(true)
  }

  return (
    <Screen>
      {back}
      <header className="workout-head">
        <h1>Завершити тренування</h1>
        <p className="muted">
          Виконано {done} з {total} {plural(total, 'підходу', 'підходів', 'підходів')}
          {different > 0 && ` · ${different} інакше, ніж у плані`}
        </p>
      </header>

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
                onClick={() => setEffort(value)}
              >
                {EFFORT_TEXT[value]}
              </button>
            ))}
          </div>
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
        <button type="submit" className="button primary block" disabled={!effort}>
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
