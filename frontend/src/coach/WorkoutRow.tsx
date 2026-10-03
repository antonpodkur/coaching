import { Link } from 'react-router'

import type { Schemas } from '../api/client'
import { formatShortDate, plural } from '../shared/format'
import type { BackTarget } from './backTarget'
import { useCoach } from './context'

type Summary = Schemas['WorkoutSummary']

const STATUS: Record<Summary['status'], { text: string; className: string }> = {
  draft: { text: 'Чернетка', className: 'tag' },
  published: { text: 'Опубліковано', className: 'tag tag-new' },
  done: { text: 'Виконано', className: 'tag tag-ok' },
}

interface Props {
  workout: Summary
  /** Replaces `title · date`, e.g. with the client's name in the workouts tab. */
  heading?: string
  /** Where the opened page's back arrow should return. */
  back?: BackTarget
}

/**
 * One workout in a list: what it is, how far the client got, and its state.
 * Opens the report once there is something to see, the builder before that.
 */
export function WorkoutRow({ workout, heading, back }: Props) {
  const { base } = useCoach()
  const hasResults = workout.done_set_count > 0 || !!workout.report
  return (
    <li>
      <Link
        className="workout-row"
        to={hasResults ? `${base}/workouts/${workout.id}/report` : `${base}/workouts/${workout.id}`}
        state={back ? { back } : undefined}
      >
        <span className="workout-text">
          <span className="workout-title">
            {heading ??
              `${workout.title || 'Без назви'} · ${workout.date ? formatShortDate(workout.date) : 'без дати'}`}
          </span>
          <span className="muted small">{workoutMeta(workout)}</span>
        </span>
        {workout.report && !workout.report.seen ? (
          <span className="tag tag-warn">Новий звіт</span>
        ) : (
          <span className={STATUS[workout.status].className}>{STATUS[workout.status].text}</span>
        )}
      </Link>
    </li>
  )
}

function workoutMeta(workout: Summary): string {
  if (workout.done_set_count === 0) {
    const size = `${workout.exercise_count} ${plural(workout.exercise_count, 'вправа', 'вправи', 'вправ')} · ${workout.set_count} ${plural(workout.set_count, 'підхід', 'підходи', 'підходів')}`
    if (workout.status !== 'published') return size
    return `${size} · ${workout.opened ? 'відкрито' : 'ще не відкрито'}`
  }
  const parts = [`${workout.done_set_count} з ${workout.set_count} підходів`]
  if (workout.different_count > 0) parts.push(`${workout.different_count} інакше`)
  if (workout.report?.has_comment) parts.push('є коментар')
  return parts.join(' · ')
}
