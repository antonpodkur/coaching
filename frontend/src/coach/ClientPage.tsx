import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError, type Schemas, api, unwrap } from '../api/client'
import { BackLink } from '../shared/BackLink'
import { formatDay, initials } from '../shared/format'
import { nutritionSummary } from '../shared/nutrition'
import { weightSummary } from '../shared/weight'
import { ChevronIcon, CopyIcon, EditIcon, PlusIcon } from '../shared/icons'
import { ArchivedNotice } from './ClientEditPage'
import { InviteCard, type ShownInvite } from './InviteCard'
import { ReportCard } from './ReportCard'
import { WorkoutRow } from './WorkoutRow'
import { clientQuery, nutritionQuery, payment, questionnaireSummary, weightQuery } from './clients'
import { CLIENTS_KEY, WORKOUTS_KEY, isUnauthorized, useCoach } from './context'
import { resultsQuery } from './results'

/** One client: their workouts, and where new ones start. */
export function ClientPage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [invite, setInvite] = useState<ShownInvite | null>(null)

  const client = useQuery(clientQuery(id))
  const weight = useQuery(weightQuery(id))
  const nutrition = useQuery(nutritionQuery(id))
  const workouts = useQuery({
    queryKey: [...WORKOUTS_KEY, 'client', id],
    queryFn: async () =>
      unwrap(await api.GET('/coach/clients/{id}/workouts', { params: { path: { id } } })),
  })
  // The newest report leads the page, the way Dasha checks in on a client.
  const reported = workouts.data?.find((workout) => workout.report)
  const report = useQuery({ ...resultsQuery(reported?.id ?? ''), enabled: reported !== undefined })
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
  // The report goes above the buttons and the list, so they wait for it: a
  // report that arrives later pushes them down, under a finger already on its way.
  const loading = workouts.isPending || (reported !== undefined && report.isPending)
  const status = person.joined
    ? 'У застосунку'
    : person.invite_expires_at && new Date(person.invite_expires_at) > new Date()
      ? `Запрошення до ${formatDay(person.invite_expires_at)}`
      : 'Ще не в застосунку'
  const paid = payment(person.paid_until)
  // An archived client gets no new workouts or invites until restored.
  const active = !person.archived

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
            {person.archived ? 'В архіві' : status}
            {paid && (
              <>
                {' · '}
                <span className={paid.due && active ? 'warn' : undefined}>{paid.text}</span>
              </>
            )}
          </span>
        </div>
        <Link
          className="icon-button"
          to={`${base}/clients/${id}/edit`}
          aria-label="Змінити дані клієнта"
        >
          <EditIcon />
        </Link>
      </header>

      {person.archived && <ArchivedNotice client={person} />}

      <Link className="workout-row" to={`${base}/clients/${id}/questionnaire`}>
        <span className="workout-text">
          <span className="workout-title">Анкета</span>
          <span className="muted small">{questionnaireSummary(person) ?? 'Ще не заповнена'}</span>
        </span>
        <ChevronIcon />
      </Link>
      <Link className="workout-row" to={`${base}/clients/${id}/weight`}>
        <span className="workout-text">
          <span className="workout-title">Вага</span>
          <span className="muted small">
            {weight.isPending ? '…' : (weightSummary(weight.data ?? []) ?? 'Ще немає записів')}
          </span>
        </span>
        <ChevronIcon />
      </Link>
      <Link className="workout-row" to={`${base}/clients/${id}/nutrition`}>
        <span className="workout-text">
          <span className="workout-title">Харчування</span>
          <span className="muted small">
            {nutrition.isPending
              ? '…'
              : nutrition.data?.targets[0]
                ? nutritionSummary(nutrition.data.targets[0])
                : 'Ще не задано'}
          </span>
        </span>
        <ChevronIcon />
      </Link>

      {active && !person.joined && !invite && (
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

      {loading && <p className="muted">Завантаження…</p>}

      {reported && report.data && (
        <LatestReport
          key={reported.id}
          results={report.data}
          onCopy={active ? () => create.mutate(reported.id) : undefined}
          copying={create.isPending}
        />
      )}

      {active && !loading && (
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
          {latest && latest.id !== reported?.id && (
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
      )}

      {!loading && (
        <section className="stack" aria-labelledby="workouts-title">
          <h2 id="workouts-title" className="section-title">
            Тренування
          </h2>
          {workouts.isError && !isUnauthorized(workouts.error) && (
            <p className="error">Не вдалося завантажити тренування.</p>
          )}
          {workouts.data?.length === 0 && (
            <p className="muted">Ще немає. Склади перше — клієнт побачить його після публікації.</p>
          )}
          <ul className="workout-list">
            {workouts.data?.map((workout) => (
              <WorkoutRow key={workout.id} workout={workout} />
            ))}
          </ul>
        </section>
      )}
    </section>
  )
}

/** The newest report, differences first, with the step Dasha usually takes next. */
function LatestReport({
  results,
  onCopy,
  copying,
}: {
  results: Schemas['WorkoutResults']
  /** Left out for an archived client. */
  onCopy?: () => void
  copying: boolean
}) {
  const { base } = useCoach()
  return (
    <ReportCard
      results={results}
      compact
      actions={
        <div className="report-actions">
          {onCopy && (
            <button type="button" className="button block" disabled={copying} onClick={onCopy}>
              <CopyIcon />
              Скопіювати в наступне тренування
            </button>
          )}
          <Link className="link-button" to={`${base}/workouts/${results.id}/report`}>
            Відкрити звіт повністю
          </Link>
        </div>
      }
    />
  )
}

