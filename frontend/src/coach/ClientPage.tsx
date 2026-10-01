import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError, type Schemas, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { formatDay, formatShortDate, initials, plural } from '../shared/format'
import { CopyIcon, PlusIcon } from '../shared/icons'
import { InviteCard, type ShownInvite } from './InviteCard'
import { CLIENTS_KEY, WORKOUTS_KEY, isUnauthorized, useCoach } from './context'

type Summary = Schemas['WorkoutSummary']

const STATUS: Record<Summary['status'], { text: string; className: string }> = {
  draft: { text: 'Чернетка', className: 'tag' },
  published: { text: 'Опубліковано', className: 'tag tag-new' },
  done: { text: 'Виконано', className: 'tag tag-ok' },
}

/** One client: their workouts, and where new ones start. */
export function ClientPage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [invite, setInvite] = useState<ShownInvite | null>(null)

  const client = useQuery({
    queryKey: [...CLIENTS_KEY, id],
    queryFn: async () =>
      unwrap(await api.GET('/coach/clients/{id}', { params: { path: { id } } })),
  })
  const workouts = useQuery({
    queryKey: [...WORKOUTS_KEY, 'client', id],
    queryFn: async () =>
      unwrap(await api.GET('/coach/clients/{id}/workouts', { params: { path: { id } } })),
  })
  useEffect(() => {
    if (isUnauthorized(client.error) || isUnauthorized(workouts.error)) onUnauthorized()
  }, [client.error, workouts.error, onUnauthorized])

  const create = useMutation({
    mutationFn: async (copyFrom?: string) =>
      unwrap(
        await api.POST('/coach/workouts', { body: { client_id: id, copy_from: copyFrom } }),
      ),
    onSuccess: (workout) => {
      void queryClient.invalidateQueries({ queryKey: WORKOUTS_KEY })
      void navigate(`${base}/workouts/${workout.id}`)
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  const reinvite = useMutation({
    mutationFn: async (name: string) => ({
      name,
      ...unwrap(await api.POST('/coach/clients/{id}/invite', { params: { path: { id } } })),
    }),
    onSuccess: (link) => {
      setInvite(link)
      void queryClient.invalidateQueries({ queryKey: CLIENTS_KEY })
    },
    onError: (err) => {
      if (isUnauthorized(err)) onUnauthorized()
    },
  })

  const back = <BackLink to={base} label="Клієнти" />
  if (client.isPending) {
    return (
      <section className="page">
        {back}
        <p className="muted">Завантаження…</p>
      </section>
    )
  }
  if (client.isError) {
    const missing = client.error instanceof ApiError && client.error.status === 404
    return (
      <section className="page">
        {back}
        <p className="error">{missing ? 'Такого клієнта немає.' : 'Не вдалося завантажити.'}</p>
      </section>
    )
  }

  const person = client.data
  // "Copy to the next one" starts from the latest workout that has something in it.
  const latest = workouts.data?.find((workout) => workout.date && workout.exercise_count > 0)
  const status = person.joined
    ? 'У застосунку'
    : person.invite_expires_at && new Date(person.invite_expires_at) > new Date()
      ? `Запрошення до ${formatDay(person.invite_expires_at)}`
      : 'Ще не в застосунку'

  return (
    <section className="page">
      {back}
      <header className="client-head">
        <span className="avatar large" aria-hidden="true">
          {initials(person.name)}
        </span>
        <div className="client-text">
          <h1>{person.name}</h1>
          <span className="client-status">
            {status}
            {person.paid_until && ` · оплачено до ${formatDay(person.paid_until)}`}
          </span>
        </div>
      </header>

      {!person.joined && !invite && (
        <button
          type="button"
          className="button block"
          disabled={reinvite.isPending}
          onClick={() => reinvite.mutate(person.name)}
        >
          Нове посилання-запрошення
        </button>
      )}
      {invite && <InviteCard invite={invite} onClose={() => setInvite(null)} />}

      <div className="stack">
        <button
          type="button"
          className="button primary block"
          disabled={create.isPending}
          onClick={() => create.mutate(undefined)}
        >
          <PlusIcon />
          Нове тренування
        </button>
        {latest && (
          <button
            type="button"
            className="button block"
            disabled={create.isPending}
            onClick={() => create.mutate(latest.id)}
          >
            <CopyIcon />
            Скопіювати «{latest.title || 'тренування'}» на наступний тиждень
          </button>
        )}
        {create.isError && !isUnauthorized(create.error) && (
          <p className="error">Не вдалося створити тренування.</p>
        )}
      </div>

      <section className="stack" aria-labelledby="workouts-title">
        <h2 id="workouts-title" className="section-title">
          Тренування
        </h2>
        {workouts.isPending && <p className="muted">Завантаження…</p>}
        {workouts.data?.length === 0 && (
          <p className="muted">Ще немає. Склади перше — клієнт побачить його після публікації.</p>
        )}
        <ul className="workout-list">
          {workouts.data?.map((workout) => (
            <li key={workout.id}>
              <Link className="workout-row" to={`${base}/workouts/${workout.id}`}>
                <span className="workout-text">
                  <span className="workout-title">
                    {workout.title || 'Без назви'}
                    {' · '}
                    {workout.date ? formatShortDate(workout.date) : 'без дати'}
                  </span>
                  <span className="muted small">
                    {workout.exercise_count}{' '}
                    {plural(workout.exercise_count, 'вправа', 'вправи', 'вправ')} ·{' '}
                    {workout.set_count} {plural(workout.set_count, 'підхід', 'підходи', 'підходів')}
                  </span>
                </span>
                <span className={STATUS[workout.status].className}>{STATUS[workout.status].text}</span>
              </Link>
            </li>
          ))}
        </ul>
      </section>
    </section>
  )
}
