import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router'

import { ApiError, api, unwrap } from '../../api/client'
import { BackLink } from '../../shared/BackLink'
import { plural } from '../../shared/format'
import { CheckIcon, PlusIcon } from '../../shared/icons'
import { useBackTarget } from '../backTarget'
import { EXERCISES_KEY, WORKOUTS_KEY, isUnauthorized, useCoach } from '../context'
import { type Exercise, confirmAction } from '../library'
import { ExerciseCard } from './ExerciseCard'
import { LibrarySheet } from './LibrarySheet'
import { SetEditor } from './SetEditor'
import { type Draft, type DraftExercise, type Workout, newSet, perSideLabel } from './draft'
import { type SaveStatus, useAutosave } from './useAutosave'

/** Loads the workout, then hands it to the builder. */
export function BuilderPage() {
  const { id = '' } = useParams()
  const { onUnauthorized } = useCoach()
  const [loads, setLoads] = useState(0)

  const workout = useQuery({
    // Outside WORKOUTS_KEY: refreshing lists must not refetch what she is editing.
    queryKey: ['coach-workout-builder', id, loads],
    queryFn: async () =>
      unwrap(await api.GET('/coach/workouts/{id}', { params: { path: { id } } })),
    // The builder keeps its own working copy; never swap it out underneath her.
    staleTime: Infinity,
    gcTime: 0,
  })
  useEffect(() => {
    if (isUnauthorized(workout.error)) onUnauthorized()
  }, [workout.error, onUnauthorized])

  if (workout.isPending) return <p className="muted">Завантаження…</p>
  if (workout.isError) {
    const missing = workout.error instanceof ApiError && workout.error.status === 404
    return <p className="error">{missing ? 'Такого тренування немає.' : 'Не вдалося завантажити тренування.'}</p>
  }
  return (
    <Builder
      key={`${workout.data.id}-${loads}`}
      workout={workout.data}
      onReload={() => setLoads((n) => n + 1)}
    />
  )
}

type Selection = { exercise: string; set: number } | null

function Builder({ workout, onReload }: { workout: Workout; onReload: () => void }) {
  const { base, onUnauthorized } = useCoach()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { draft, update, status, flush } = useAutosave(workout, onUnauthorized)
  const [selection, setSelection] = useState<Selection>(null)
  const [sheetOpen, setSheetOpen] = useState(false)
  const [published, setPublished] = useState({
    status: workout.status,
    notified: null as boolean | null,
  })

  // Muscle groups decide the default "per arm / per leg" label.
  const library = useQuery({
    queryKey: EXERCISES_KEY,
    queryFn: async () => unwrap(await api.GET('/coach/exercises')),
  })
  const groupOf = (exerciseId: string) =>
    library.data?.find((exercise) => exercise.id === exerciseId)?.muscle_group

  const clientPath = workout.client_id ? `${base}/clients/${workout.client_id}` : base
  const back = useBackTarget({ to: clientPath, label: 'Клієнт' })
  const setCount = draft.exercises.reduce((total, exercise) => total + exercise.sets.length, 0)

  const changeExercise = (id: string, change: (exercise: DraftExercise) => DraftExercise) =>
    update((current: Draft) => ({
      ...current,
      exercises: current.exercises.map((exercise) => (exercise.id === id ? change(exercise) : exercise)),
    }))

  const addExercise = (picked: Exercise) => {
    const row: DraftExercise = {
      id: crypto.randomUUID(),
      exercise_id: picked.id,
      name: picked.name,
      measure: picked.measure,
      thumbnail_url: picked.video?.thumbnail_url ?? null,
      per_side_label: null,
      note: null,
      sets: [newSet(picked.measure)],
      last_time: '',
    }
    update((current) => ({ ...current, exercises: [...current.exercises, row] }))
    setSheetOpen(false)
    setSelection({ exercise: row.id, set: 0 })
  }

  const moveExercise = (index: number, delta: -1 | 1) =>
    update((current) => {
      const exercises = [...current.exercises]
      const [moved] = exercises.splice(index, 1)
      if (!moved) return current
      exercises.splice(index + delta, 0, moved)
      return { ...current, exercises }
    })

  const removeExercise = async (exercise: DraftExercise) => {
    const confirmed = await confirmAction(`Прибрати «${exercise.name}» з тренування?`)
    if (!confirmed) return
    if (selection?.exercise === exercise.id) setSelection(null)
    update((current) => ({
      ...current,
      exercises: current.exercises.filter((row) => row.id !== exercise.id),
    }))
  }

  const publish = useMutation({
    mutationFn: async () => {
      if (!(await flush())) throw new Error('not saved')
      return unwrap(
        await api.POST('/coach/workouts/{id}/publish', { params: { path: { id: workout.id } } }),
      )
    },
    onSuccess: (result) => {
      setPublished({ status: result.workout.status, notified: result.client_notified })
      void queryClient.invalidateQueries({ queryKey: WORKOUTS_KEY })
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  const deleteWorkout = useMutation({
    mutationFn: async () =>
      unwrap(await api.DELETE('/coach/workouts/{id}', { params: { path: { id: workout.id } } })),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: WORKOUTS_KEY })
      void navigate(clientPath, { replace: true })
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  const selectedExercise = selection
    ? draft.exercises.find((exercise) => exercise.id === selection.exercise)
    : undefined
  const editingSet = selection ? selectedExercise?.sets[selection.set] : undefined
  const editing = editingSet ? selection : null
  const isPublished = published.status !== 'draft'
  const canPublish = draft.date !== null && draft.exercises.length > 0

  return (
    <section className={editing ? 'page builder editing' : 'page builder'}>
      <div className="builder-top">
        <BackLink to={back.to} label={back.label} />
        {isPublished ? (
          <span className="tag tag-ok publish-state">
            <CheckIcon size={12} />
            Опубліковано
          </span>
        ) : (
          <button
            type="button"
            className="button primary small publish-state"
            disabled={!canPublish || publish.isPending || status === 'conflict'}
            onClick={() => publish.mutate()}
          >
            {publish.isPending ? 'Публікую…' : 'Опублікувати'}
          </button>
        )}
      </div>

      <div className="builder-head">
        <input
          className="builder-title"
          aria-label="Назва тренування"
          placeholder="Назва, напр. Спина"
          value={draft.title}
          maxLength={120}
          onChange={(event) => update((current) => ({ ...current, title: event.target.value }))}
        />
        <div className="builder-meta">
          <label className="date-chip">
            <span className="sr-only">Дата</span>
            <input
              type="date"
              value={draft.date ?? ''}
              onChange={(event) =>
                update((current) => ({ ...current, date: event.target.value || null }))
              }
            />
          </label>
          <span className="muted small">
            {draft.exercises.length} {plural(draft.exercises.length, 'вправа', 'вправи', 'вправ')} ·{' '}
            {setCount} {plural(setCount, 'підхід', 'підходи', 'підходів')}
          </span>
          <SaveIndicator status={status} onReload={onReload} />
        </div>
        {!isPublished && !canPublish && (
          <p className="muted small">Щоб опублікувати, вкажи дату й додай вправи.</p>
        )}
        {publish.isError && !isUnauthorized(publish.error) && (
          <p className="error">{publishError(publish.error)}</p>
        )}
        {isPublished && (
          <p className="notice small">
            {published.notified === false
              ? 'Опубліковано. Бот не написав клієнту: той ще не приєднався або не дозволив повідомлення. Тренування вже в застосунку.'
              : 'Опубліковано. Зміни клієнт бачить одразу.'}
          </p>
        )}
      </div>

      {draft.exercises.length === 0 && (
        <p className="muted">Додай першу вправу з бібліотеки. Підходи з’являться як картки, які можна редагувати.</p>
      )}

      <ol className="builder-list">
        {draft.exercises.map((exercise, index) => (
          <ExerciseCard
            key={exercise.id}
            exercise={exercise}
            index={index}
            count={draft.exercises.length}
            selectedSet={editing?.exercise === exercise.id ? editing.set : null}
            onSelectSet={(set) => setSelection({ exercise: exercise.id, set })}
            onAddSet={() => {
              changeExercise(exercise.id, (row) => ({
                ...row,
                sets: [...row.sets, newSet(row.measure, row.sets.at(-1))],
              }))
              setSelection({ exercise: exercise.id, set: exercise.sets.length })
            }}
            onMove={(delta) => moveExercise(index, delta)}
            onNote={(note) => changeExercise(exercise.id, (row) => ({ ...row, note }))}
            onRemove={() => void removeExercise(exercise)}
          />
        ))}
      </ol>

      <div className="builder-footer">
        <button
          type="button"
          className="link-button danger"
          disabled={deleteWorkout.isPending}
          onClick={() =>
            void confirmAction('Видалити це тренування?').then(
              (confirmed) => confirmed && deleteWorkout.mutate(),
            )
          }
        >
          Видалити тренування
        </button>
      </div>

      {editing && selectedExercise && editingSet ? (
        <SetEditor
          key={editingSet.id}
          exercise={selectedExercise}
          set={editingSet}
          setIndex={editing.set}
          sideLabel={perSideLabel(groupOf(selectedExercise.exercise_id))}
          onChange={(set) =>
            changeExercise(selectedExercise.id, (row) => ({
              ...row,
              sets: row.sets.map((current) => (current.id === set.id ? set : current)),
            }))
          }
          onToggleSide={() =>
            changeExercise(selectedExercise.id, (row) => ({
              ...row,
              per_side_label:
                row.per_side_label === null ? perSideLabel(groupOf(row.exercise_id)) : null,
            }))
          }
          onCopyToAll={() =>
            changeExercise(selectedExercise.id, (row) => ({
              ...row,
              sets: row.sets.map((set) => ({ ...editingSet, id: set.id })),
            }))
          }
          onRemove={() => {
            changeExercise(selectedExercise.id, (row) => ({
              ...row,
              sets: row.sets.filter((_, index) => index !== editing.set),
            }))
            setSelection(null)
          }}
          onDone={() => setSelection(null)}
        />
      ) : (
        <div className="builder-add">
          <button type="button" className="button block" onClick={() => setSheetOpen(true)}>
            <PlusIcon />
            Вправа з бібліотеки
          </button>
        </div>
      )}

      {sheetOpen && (
        <LibrarySheet
          used={new Set(draft.exercises.map((exercise) => exercise.exercise_id))}
          onPick={addExercise}
          onClose={() => setSheetOpen(false)}
        />
      )}
    </section>
  )
}

function SaveIndicator({ status, onReload }: { status: SaveStatus; onReload: () => void }) {
  switch (status) {
    case 'saved':
      return (
        <span className="save-status">
          <CheckIcon size={14} />
          Збережено
        </span>
      )
    case 'pending':
    case 'saving':
      return <span className="save-status">Зберігаю…</span>
    case 'error':
      return <span className="save-status warn">Не збережено, пробую ще</span>
    case 'conflict':
      return (
        <button type="button" className="save-status warn" onClick={onReload}>
          Змінено на іншому пристрої · оновити
        </button>
      )
  }
}

function publishError(error: Error): string {
  if (!(error instanceof ApiError)) return 'Спершу треба зберегти зміни. Перевір зв’язок і спробуй ще.'
  switch (error.code) {
    case 'date_required':
      return 'Вкажи дату тренування.'
    case 'empty_workout':
      return 'Додай хоча б одну вправу.'
    case 'exercise_without_sets':
      return 'У кожної вправи має бути хоча б один підхід.'
    default:
      return 'Не вдалося опублікувати. Спробуй ще раз.'
  }
}
