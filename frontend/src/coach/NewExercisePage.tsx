import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useNavigate } from 'react-router'

import { ApiError, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { GroupPicker } from './GroupPicker'
import { MeasurePicker } from './MeasurePicker'
import { EXERCISES_KEY, isUnauthorized, useCoach } from './context'
import type { Measure } from './library'

/** Adds an exercise to the library; the video comes on the next screen. */
export function NewExercisePage() {
  const { base, onUnauthorized } = useCoach()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [name, setName] = useState('')
  const [group, setGroup] = useState<string | null>(null)
  const [measure, setMeasure] = useState<Measure>('weight')

  const create = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.POST('/coach/exercises', {
          body: { name, muscle_group: group ?? undefined, measure },
        }),
      ),
    onSuccess: (exercise) => {
      void queryClient.invalidateQueries({ queryKey: EXERCISES_KEY })
      void navigate(`${base}/exercises/${exercise.id}`, { replace: true })
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })
  const taken = create.error instanceof ApiError && create.error.code === 'name_taken'

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (name.trim()) create.mutate()
  }

  return (
    <section className="page">
      <BackLink to={`${base}/exercises`} label="Вправи" />
      <div className="page-intro">
        <h1>Нова вправа</h1>
        <p className="muted">Відео техніки можна додати одразу після цього або пізніше.</p>
      </div>
      <form className="stack" onSubmit={submit}>
        <label className="field">
          <span>Назва</span>
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Наприклад, Румунська тяга"
            maxLength={120}
          />
        </label>
        <div className="field">
          <span>Група м’язів</span>
          <GroupPicker value={group} onChange={setGroup} />
        </div>
        <div className="field">
          <span>Як рахувати</span>
          <MeasurePicker value={measure} onChange={setMeasure} />
        </div>
        <button
          type="submit"
          className="button primary block"
          disabled={!name.trim() || create.isPending}
        >
          {create.isPending ? 'Додаю…' : 'Додати вправу'}
        </button>
        {taken && <p className="error">Така вправа вже є в бібліотеці.</p>}
        {create.isError && !taken && !isUnauthorized(create.error) && (
          <p className="error">Не вдалося додати. Спробуй ще раз.</p>
        )}
      </form>
    </section>
  )
}
