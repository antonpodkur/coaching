import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { Link } from 'react-router'

import { api, unwrap } from '../api/client'
import { CameraIcon, PlusIcon, SearchIcon } from '../shared/icons'
import { GroupChip } from './GroupPicker'
import { UploadCard } from './UploadCard'
import { EXERCISES_KEY, isUnauthorized, useCoach } from './context'
import { type Exercise, videoSummary } from './library'
import { useUploads } from './uploads'

/** Encoding happens on Bunny's side; check back this often while it runs. */
const PROCESSING_POLL_MS = 10_000

/** The exercise library: what she builds workouts from, with her videos. */
export function LibraryPage() {
  const { base, onUnauthorized } = useCoach()
  const [query, setQuery] = useState('')
  const [group, setGroup] = useState<string | null>(null)
  const uploads = useUploads()

  const exercises = useQuery({
    queryKey: EXERCISES_KEY,
    queryFn: async () => unwrap(await api.GET('/coach/exercises')),
    refetchInterval: (q) =>
      q.state.data?.some((exercise) => exercise.upload?.status === 'processing')
        ? PROCESSING_POLL_MS
        : false,
  })
  useEffect(() => {
    if (isUnauthorized(exercises.error)) onUnauthorized()
  }, [exercises.error, onUnauthorized])

  const all = exercises.data ?? []
  const groups = [...new Set(all.map((e) => e.muscle_group).filter((g): g is string => !!g))]
  const needle = query.trim().toLocaleLowerCase('uk')
  const shown = all.filter(
    (exercise) =>
      (!group || exercise.muscle_group === group) &&
      (!needle ||
        [exercise.name, ...exercise.aliases].some((name) =>
          name.toLocaleLowerCase('uk').includes(needle),
        )),
  )

  return (
    <section className="page">
      <header className="page-head">
        <div className="page-title">
          <p className="muted small">Бібліотека</p>
          <h1>
            Вправи {exercises.data && <span className="count">{all.length}</span>}
          </h1>
        </div>
        <Link className="button primary" to={`${base}/exercises/new`}>
          <PlusIcon />
          Нова
        </Link>
      </header>

      {uploads.map((upload) => (
        <UploadCard key={upload.exerciseId} upload={upload} linkToExercise />
      ))}

      {all.length > 0 && (
        <label className="search">
          <SearchIcon />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Знайти вправу"
            aria-label="Знайти вправу"
          />
        </label>
      )}

      {groups.length > 1 && (
        <div className="chips" role="group" aria-label="Група м’язів">
          <GroupChip label="Усі" selected={group === null} onClick={() => setGroup(null)} />
          {groups.map((name) => (
            <GroupChip
              key={name}
              label={name}
              selected={group === name}
              onClick={() => setGroup(name)}
            />
          ))}
        </div>
      )}

      {exercises.isPending && <p className="muted">Завантаження…</p>}
      {exercises.isError && !isUnauthorized(exercises.error) && (
        <p className="error">Не вдалося завантажити бібліотеку.</p>
      )}
      {exercises.data?.length === 0 && (
        <p className="muted">
          Бібліотека порожня. Додай вправу, а відео техніки можна завантажити одразу або пізніше.
        </p>
      )}
      {all.length > 0 && shown.length === 0 && <p className="muted">Нічого не знайдено.</p>}

      <ul className="exercise-list">
        {shown.map((exercise) => (
          <li key={exercise.id}>
            <ExerciseRow
              exercise={exercise}
              href={`${base}/exercises/${exercise.id}`}
              upload={uploads.find((upload) => upload.exerciseId === exercise.id)}
            />
          </li>
        ))}
      </ul>
    </section>
  )
}

function ExerciseRow({
  exercise,
  href,
  upload,
}: {
  exercise: Exercise
  href: string
  upload: ReturnType<typeof useUploads>[number] | undefined
}) {
  const summary = videoSummary(exercise, upload)
  return (
    <Link className="exercise-row" to={href}>
      {exercise.video ? (
        <img className="thumb" src={exercise.video.thumbnail_url} alt="" loading="lazy" />
      ) : (
        <span className="thumb thumb-empty" aria-hidden="true">
          <CameraIcon />
        </span>
      )}
      <span className="exercise-text">
        <span className="exercise-name">{exercise.name}</span>
        <span className={summary.busy ? 'exercise-meta busy' : 'exercise-meta'}>
          {summary.text}
        </span>
      </span>
    </Link>
  )
}
