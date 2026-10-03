import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useEffect, useState } from 'react'

import { type Schemas, api, unwrap } from '../api/client'
import {
  type Measure,
  formatCount,
  formatDone,
  formatShortDate,
  formatTarget,
  formatWhen,
  plural,
} from '../shared/format'
import { FormVideoTile } from '../shared/FormVideoTile'
import { CLIENTS_KEY, RESULTS_KEY, WORKOUTS_KEY } from './context'

type Results = Schemas['WorkoutResults']
type ResultSet = Schemas['ResultSet']
type ResultExercise = Schemas['ResultExercise']

const EFFORT: Record<Schemas['Effort'], { text: string; className: string }> = {
  easy: { text: 'Легко', className: 'tag tag-ok' },
  ok: { text: 'Нормально', className: 'tag' },
  hard: { text: 'Важко', className: 'tag tag-warn' },
}

/** `36 × 10 · план 12`, `34 × 12 · план 36 × 12`, `40 с · план 45 с`, or `пропущено`. */
function resultLabel(set: ResultSet, measure: Measure): string {
  if (!set.completed) return 'пропущено'
  const done = formatDone(set.actual_kg, set.actual_reps, measure)
  if (!set.differs) return done
  const kgDiffers = (set.actual_kg ?? null) !== (set.target_kg ?? null)
  const plan = kgDiffers
    ? formatTarget(set.target_kg, set.target_reps_min, set.target_reps_max, measure)
    : formatCount(set.target_reps_min, set.target_reps_max, measure)
  return `${done} · план ${plan}`
}

/** Something to look at: a set done differently or not done, or a video. */
function standsOut(exercise: ResultExercise): boolean {
  return exercise.videos.length > 0 || exercise.sets.some((set) => set.differs || !set.completed)
}

function hasNewVideo(results: Results): boolean {
  return results.exercises.some((exercise) =>
    exercise.videos.some((video) => video.status === 'ready' && !video.seen),
  )
}

interface Props {
  results: Results
  /** Show what differed first, the rest behind a toggle (the client page). */
  compact?: boolean
  actions?: ReactNode
}

/**
 * A workout's report for Dasha: what the client did next to the plan, how it
 * felt and their comment. Opening a new report marks it seen.
 */
export function ReportCard({ results, compact = false, actions }: Props) {
  const queryClient = useQueryClient()
  // Keep the "new" label while she reads, even after it is marked seen.
  const [isNew] = useState(results.report?.seen === false)
  // Opening the report also sees the client's new videos.
  const [toMark] = useState(isNew || hasNewVideo(results))
  const [showAll, setShowAll] = useState(!compact)

  const markSeen = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.POST('/coach/workouts/{id}/report/seen', {
          params: { path: { id: results.id } },
        }),
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: CLIENTS_KEY })
      void queryClient.invalidateQueries({ queryKey: WORKOUTS_KEY })
      // This card keeps its label; other views of the report should say "seen".
      void queryClient.invalidateQueries({ queryKey: RESULTS_KEY })
    },
  })
  const { mutate } = markSeen
  useEffect(() => {
    if (toMark) mutate()
  }, [toMark, mutate])

  let total = 0
  let done = 0
  let different = 0
  for (const exercise of results.exercises) {
    for (const set of exercise.sets) {
      total += 1
      if (set.completed) done += 1
      if (set.differs) different += 1
    }
  }
  const notable = results.exercises.filter(standsOut)
  const shown = showAll ? results.exercises : notable
  const report = results.report

  const label = report
    ? `${isNew ? 'Новий звіт' : 'Звіт'} · ${formatWhen(report.finished_at)}`
    : done > 0
      ? 'Ще триває'
      : 'Ще не почато'
  const meta = [
    report?.duration_min != null && `${report.duration_min} хв`,
    `${done} з ${total} ${plural(total, 'підходу', 'підходів', 'підходів')}`,
    different > 0 && `${different} інакше, ніж у плані`,
  ]
    .filter(Boolean)
    .join(' · ')

  return (
    <section className={isNew ? 'report new' : 'report'} aria-label="Звіт">
      <div className="report-head">
        <span className="report-label">{label}</span>
        <h2>
          {results.title || 'Тренування'}
          {results.date && ` · ${formatShortDate(results.date)}`}
        </h2>
        <span className="muted small">{meta}</span>
      </div>

      {report && (
        <div className="report-feedback">
          <span className={EFFORT[report.effort].className}>{EFFORT[report.effort].text}</span>
          {report.comment && <p>«{report.comment}»</p>}
        </div>
      )}

      <div className="report-rows">
        <span className="section-title">
          {showAll
            ? 'Усі вправи'
            : notable.some((exercise) => exercise.videos.length > 0)
              ? 'Варто переглянути'
              : notable.length > 0
                ? 'Інакше, ніж у плані'
                : 'Усе за планом'}
        </span>
        {shown.map((exercise) => (
          <div key={exercise.id} className="report-row">
            <span className="report-exercise">
              {exercise.name}
              {exercise.per_side_label && (
                <span className="muted small"> · {exercise.per_side_label}</span>
              )}
            </span>
            <span className="result-chips">
              {exercise.sets.map((set) => (
                <span
                  key={set.id}
                  className={
                    !set.completed ? 'result-chip skipped' : set.differs ? 'result-chip differs' : 'result-chip'
                  }
                >
                  {resultLabel(set, exercise.measure)}
                </span>
              ))}
            </span>
            {exercise.videos.length > 0 && (
              <div className="form-videos">
                {exercise.videos.map((video, index) => (
                  <FormVideoTile
                    key={video.id}
                    video={video}
                    label={`Відео ${index + 1}${video.status === 'ready' && !video.seen ? ' · нове' : ''}`}
                  />
                ))}
              </div>
            )}
          </div>
        ))}
        {compact && results.exercises.length > notable.length && (
          <button
            type="button"
            className="link-button"
            aria-expanded={showAll}
            onClick={() => setShowAll(!showAll)}
          >
            {showAll
              ? 'Лише відмінності'
              : `Показати всі ${results.exercises.length} ${plural(results.exercises.length, 'вправу', 'вправи', 'вправ')}`}
          </button>
        )}
      </div>

      {actions}
    </section>
  )
}
