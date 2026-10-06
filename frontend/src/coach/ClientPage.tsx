import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { AnimatePresence } from 'motion/react'
import { useEffect, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError, type Schemas, api, unwrap } from '../api/client'
import { Avatar } from '../shared/Avatar'
import { BackLink } from '../shared/BackLink'
import { Sheet } from '../shared/Sheet'
import { addDays, formatDay, formatShortDate } from '../shared/format'
import { nutritionSummary } from '../shared/nutrition'
import { weightSummary } from '../shared/weight'
import {
  ChevronIcon,
  CopyIcon,
  EditIcon,
  PersonIcon,
  PlateIcon,
  PlusIcon,
  ScaleIcon,
} from '../shared/icons'
import { ArchivedNotice } from './ClientEditPage'
import { InviteCard, type ShownInvite } from './InviteCard'
import { ReportCard } from './ReportCard'
import { WorkoutRow } from './WorkoutRow'
import {
  clientQuery,
  nutritionQuery,
  payment,
  planned,
  questionnaireSummary,
  weightQuery,
} from './clients'
import { CLIENTS_KEY, WORKOUTS_KEY, isUnauthorized, useCoach } from './context'
import { resultsQuery } from './results'

/** One client: their workouts, and where new ones start. */
export function ClientPage() {
  const { id = '' } = useParams()
  const { base, onUnauthorized } = useCoach()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [invite, setInvite] = useState<ShownInvite | null>(null)
  const [choosing, setChoosing] = useState(false)

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
  // The next workout starts as a copy of one of these, a week later, or empty.
  const sources = [reported, latest].filter(
    (workout, index, all): workout is Schemas['WorkoutSummary'] =>
      workout !== undefined &&
      workout.date !== null &&
      workout.exercise_count > 0 &&
      all.findIndex((other) => other?.id === workout.id) === index,
  )
  // The report goes above the buttons and the list, so they wait for it: a
  // report that arrives later pushes them down, under a finger already on its way.
  const loading = workouts.isPending || (reported !== undefined && report.isPending)
  const plan = person.joined && !person.archived ? planned(person) : null
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
        <Avatar name={person.name} url={person.avatar_url} large />
        <div className="client-text">
          <h1>{person.name}</h1>
          <span className="client-status">
            {person.archived ? (
              'В архіві'
            ) : plan ? (
              <span className={plan.warn ? 'warn' : undefined}>{plan.text}</span>
            ) : (
              status
            )}
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

      <ul className="list-group">
        <li>
          <Link className="list-row" to={`${base}/clients/${id}/questionnaire`}>
            <span className="list-icon" aria-hidden="true">
              <PersonIcon size={18} />
            </span>
            <span className="list-text">
              <span className="list-title">Анкета</span>
              <span className="muted small">
                {questionnaireSummary(person) ?? 'Ще не заповнена'}
              </span>
            </span>
            <ChevronIcon />
          </Link>
        </li>
        <li>
          <Link className="list-row" to={`${base}/clients/${id}/weight`}>
            <span className="list-icon" aria-hidden="true">
              <ScaleIcon size={18} />
            </span>
            <span className="list-text">
              <span className="list-title">Вага</span>
              <span className="muted small">
                {weight.isPending ? '…' : (weightSummary(weight.data ?? []) ?? 'Ще немає записів')}
              </span>
            </span>
            <ChevronIcon />
          </Link>
        </li>
        <li>
          <Link className="list-row" to={`${base}/clients/${id}/nutrition`}>
            <span className="list-icon" aria-hidden="true">
              <PlateIcon size={18} />
            </span>
            <span className="list-text">
              <span className="list-title">Харчування</span>
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
        </li>
      </ul>

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

      {reported && report.data && <LatestReport key={reported.id} results={report.data} />}

      {active && !loading && (
        <div className="stack">
          <button
            type="button"
            className="button primary block"
            disabled={create.isPending}
            onClick={() => (sources.length > 0 ? setChoosing(true) : create.mutate(undefined))}
          >
            <PlusIcon />
            Наступне тренування
          </button>
          {create.isError && !isUnauthorized(create.error) && (
            <p className="error">Не вдалося створити тренування.</p>
          )}
        </div>
      )}

      <AnimatePresence>
        {choosing && (
          <Sheet
            title="Наступне тренування"
            titleId="next-workout-title"
            fit
            onClose={() => setChoosing(false)}
          >
            <ul className="sheet-options">
              {sources.map((source) => (
                <li key={source.id}>
                  <button
                    type="button"
                    className="sheet-option"
                    disabled={create.isPending}
                    onClick={() => create.mutate(source.id)}
                  >
                    <span className="list-icon" aria-hidden="true">
                      <CopyIcon size={18} />
                    </span>
                    <span className="list-text">
                      <span className="list-title">Копія «{source.title || 'Тренування'}»</span>
                      <span className="muted small">
                        На {formatShortDate(addDays(source.date ?? '', 7))}
                        {source.id === reported?.id && ' · є звіт'}
                      </span>
                    </span>
                  </button>
                </li>
              ))}
              <li>
                <button
                  type="button"
                  className="sheet-option"
                  disabled={create.isPending}
                  onClick={() => create.mutate(undefined)}
                >
                  <span className="list-icon" aria-hidden="true">
                    <PlusIcon size={18} />
                  </span>
                  <span className="list-text">
                    <span className="list-title">Порожнє</span>
                    <span className="muted small">Скласти з нуля</span>
                  </span>
                </button>
              </li>
            </ul>
          </Sheet>
        )}
      </AnimatePresence>

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

/** The newest report, differences first. */
function LatestReport({ results }: { results: Schemas['WorkoutResults'] }) {
  const { base } = useCoach()
  return (
    <ReportCard
      results={results}
      compact
      actions={
        <div className="report-actions">
          <Link className="link-button" to={`${base}/workouts/${results.id}/report`}>
            Відкрити звіт повністю
          </Link>
        </div>
      }
    />
  )
}
