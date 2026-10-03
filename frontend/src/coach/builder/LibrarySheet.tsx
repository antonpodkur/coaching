import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'

import { ApiError, api, unwrap } from '../../api/client'
import { CameraIcon, CloseIcon, PlusIcon, SearchIcon } from '../../shared/icons'
import { GroupChip } from '../GroupPicker'
import { EXERCISES_KEY, isUnauthorized, useCoach } from '../context'
import { type Exercise, measureNote } from '../library'

interface Props {
  /** Exercises already in the workout, marked in the list. */
  used: Set<string>
  onPick: (exercise: Exercise) => void
  onClose: () => void
}

/**
 * The library as a bottom sheet: search, filter by group, tap to add. A name
 * that is not in the library yet can be added on the spot; its video can come
 * later.
 */
export function LibrarySheet({ used, onPick, onClose }: Props) {
  const { onUnauthorized } = useCoach()
  const queryClient = useQueryClient()
  const [query, setQuery] = useState('')
  const [group, setGroup] = useState<string | null>(null)
  const dialog = useRef<HTMLDivElement>(null)

  const library = useQuery({
    queryKey: EXERCISES_KEY,
    queryFn: async () => unwrap(await api.GET('/coach/exercises')),
  })
  useEffect(() => {
    if (isUnauthorized(library.error)) onUnauthorized()
  }, [library.error, onUnauthorized])

  const create = useMutation({
    mutationFn: async (name: string) =>
      unwrap(await api.POST('/coach/exercises', { body: { name } })),
    onSuccess: (exercise) => {
      void queryClient.invalidateQueries({ queryKey: EXERCISES_KEY })
      onPick(exercise)
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    dialog.current?.addEventListener('keydown', onKey)
    const node = dialog.current
    return () => node?.removeEventListener('keydown', onKey)
  }, [onClose])

  const all = library.data ?? []
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
  const exact = all.some((exercise) => exercise.name.toLocaleLowerCase('uk') === needle)
  const taken = create.error instanceof ApiError && create.error.code === 'name_taken'

  return (
    <div className="sheet-backdrop" onClick={onClose}>
      <div
        ref={dialog}
        className="sheet"
        role="dialog"
        aria-modal="true"
        aria-labelledby="library-sheet-title"
        onClick={(event) => event.stopPropagation()}
      >
        <span className="sheet-handle" aria-hidden="true" />
        <div className="sheet-head">
          <h2 id="library-sheet-title">Додати вправу</h2>
          <button type="button" className="icon-button filled" aria-label="Закрити" onClick={onClose}>
            <CloseIcon />
          </button>
        </div>
        <label className="search">
          <SearchIcon />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Знайти в бібліотеці"
            aria-label="Знайти в бібліотеці"
          />
        </label>
        {groups.length > 1 && (
          <div className="chips" role="group" aria-label="Група м’язів">
            <GroupChip label="Усі" selected={group === null} onClick={() => setGroup(null)} />
            {groups.map((name) => (
              <GroupChip key={name} label={name} selected={group === name} onClick={() => setGroup(name)} />
            ))}
          </div>
        )}

        <ul className="sheet-list">
          {library.isPending && <li className="muted">Завантаження…</li>}
          {shown.map((exercise) => (
            <li key={exercise.id}>
              <button type="button" className="sheet-item" onClick={() => onPick(exercise)}>
                {exercise.video ? (
                  <img className="thumb" src={exercise.video.thumbnail_url} alt="" loading="lazy" />
                ) : (
                  <span className="thumb thumb-empty" aria-hidden="true">
                    <CameraIcon />
                  </span>
                )}
                <span className="exercise-text">
                  <span className="exercise-name">{exercise.name}</span>
                  <span className="exercise-meta">
                    {[
                      exercise.video ? 'з відео' : 'без відео',
                      exercise.muscle_group?.toLocaleLowerCase('uk'),
                      measureNote(exercise.measure),
                      used.has(exercise.id) && 'уже в тренуванні',
                    ]
                      .filter(Boolean)
                      .join(' · ')}
                  </span>
                </span>
                <span className="add-mark" aria-hidden="true">
                  <PlusIcon size={14} />
                </span>
              </button>
            </li>
          ))}
        </ul>

        {needle && !exact && (
          <button
            type="button"
            className="button block dashed"
            disabled={create.isPending}
            onClick={() => create.mutate(query)}
          >
            <PlusIcon />
            {create.isPending ? 'Додаю…' : `Нова вправа «${query.trim()}» · відео пізніше`}
          </button>
        )}
        {taken && <p className="error">Така вправа вже є.</p>}
        {create.isError && !taken && !isUnauthorized(create.error) && (
          <p className="error">Не вдалося додати вправу.</p>
        )}
      </div>
    </div>
  )
}
