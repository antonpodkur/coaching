import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useLocation, useSearchParams } from 'react-router'

import { type Schemas, api, unwrap } from '../api/client'
import {
  addDays,
  formatRange,
  formatShortDate,
  formatToday,
  localDate,
  startOfWeek,
} from '../shared/format'
import { BackIcon, ChevronIcon } from '../shared/icons'
import { WorkoutRow } from './WorkoutRow'
import { CLIENTS_KEY, WORKOUTS_KEY, isUnauthorized, useCoach } from './context'

type Scheduled = Schemas['ScheduledWorkout']

const STATUSES = [
  { value: 'all', label: 'Усі' },
  { value: 'draft', label: 'Чернетки' },
  { value: 'published', label: 'Опубліковані' },
  { value: 'done', label: 'Виконані' },
  { value: 'new_report', label: 'Нові звіти' },
] as const
type StatusFilter = (typeof STATUSES)[number]['value']

function matches(item: Scheduled, status: StatusFilter): boolean {
  const { workout } = item
  if (status === 'all') return true
  if (status === 'new_report') return !!workout.report && !workout.report.seen
  return workout.status === status
}

/**
 * Every client's workouts, a week at a time: what is planned, published, done
 * or waiting for Dasha to read. The client and status filters live in the URL,
 * so coming back from a workout keeps them.
 */
export function WorkoutsPage() {
  const { onUnauthorized } = useCoach()
  const { pathname, search } = useLocation()
  const [params, setParams] = useSearchParams()
  const today = localDate(new Date())
  const thisWeek = startOfWeek(today)
  const weekStart = params.get('week') ?? thisWeek
  const weekEnd = addDays(weekStart, 6)
  const clientId = params.get('client') ?? ''
  const status = (STATUSES.find((option) => option.value === params.get('status'))?.value ??
    'all') satisfies StatusFilter

  const change = (key: string, value: string | null) =>
    setParams(
      (current) => {
        const next = new URLSearchParams(current)
        if (value === null) next.delete(key)
        else next.set(key, value)
        return next
      },
      { replace: true },
    )

  const clients = useQuery({
    queryKey: CLIENTS_KEY,
    queryFn: async () => unwrap(await api.GET('/coach/clients')),
  })
  const workouts = useQuery({
    queryKey: [...WORKOUTS_KEY, 'week', weekStart, clientId],
    queryFn: async () =>
      unwrap(
        await api.GET('/coach/workouts', {
          params: { query: { from: weekStart, to: weekEnd, client_id: clientId || undefined } },
        }),
      ),
  })
  useEffect(() => {
    if (isUnauthorized(workouts.error) || isUnauthorized(clients.error)) onUnauthorized()
  }, [workouts.error, clients.error, onUnauthorized])

  const shown = (workouts.data ?? []).filter((item) => matches(item, status))
  const undated = shown.filter((item) => !item.workout.date)
  const days = Array.from({ length: 7 }, (_, offset) => addDays(weekStart, offset))
    .map((date) => ({ date, items: shown.filter((item) => item.workout.date === date) }))
    .filter((day) => day.items.length > 0)
  const back = { to: `${pathname}${search}`, label: 'Тренування' }
  const sortedClients = [...(clients.data ?? [])].sort((a, b) => a.name.localeCompare(b.name, 'uk'))

  return (
    <section className="page">
      <header className="page-head">
        <div className="page-title">
          <p className="muted small">{formatToday()}</p>
          <h1>Тренування</h1>
        </div>
      </header>

      <div className="week-nav">
        <button
          type="button"
          className="icon-button filled"
          aria-label="Попередній тиждень"
          onClick={() => change('week', addDays(weekStart, -7))}
        >
          <BackIcon size={18} />
        </button>
        <div className="week-label">
          <strong>{formatRange(weekStart, weekEnd)}</strong>
          {weekStart === thisWeek ? (
            <span className="muted small">цей тиждень</span>
          ) : (
            <button type="button" className="link-button" onClick={() => change('week', null)}>
              до цього тижня
            </button>
          )}
        </div>
        <button
          type="button"
          className="icon-button filled"
          aria-label="Наступний тиждень"
          onClick={() => change('week', addDays(weekStart, 7))}
        >
          <ChevronIcon size={18} />
        </button>
      </div>

      <select
        className="filter-select"
        aria-label="Клієнт"
        value={clientId}
        onChange={(event) => change('client', event.target.value || null)}
      >
        <option value="">Усі клієнти</option>
        {sortedClients.map((client) => (
          <option key={client.id} value={client.id}>
            {client.name}
          </option>
        ))}
      </select>

      <div className="chips" role="group" aria-label="Стан">
        {STATUSES.map((option) => (
          <button
            key={option.value}
            type="button"
            className="chip"
            aria-pressed={status === option.value}
            onClick={() => change('status', option.value === 'all' ? null : option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>

      {workouts.isPending && <p className="muted">Завантаження…</p>}
      {workouts.isError && !isUnauthorized(workouts.error) && (
        <p className="error">Не вдалося завантажити тренування.</p>
      )}
      {workouts.isSuccess && shown.length === 0 && (
        <p className="muted">
          {status === 'all' ? 'Цього тижня тренувань немає.' : 'Нічого не знайдено.'}
        </p>
      )}

      {undated.length > 0 && (
        <section className="day-group" aria-label="Без дати">
          <h2 className="section-title">Без дати</h2>
          <ul className="workout-list">
            {undated.map((item) => (
              <WorkoutRow
                key={item.workout.id}
                workout={item.workout}
                heading={`${item.client_name} · ${item.workout.title || 'Без назви'}`}
                back={back}
              />
            ))}
          </ul>
        </section>
      )}

      {days.map((day) => (
        <section key={day.date} className="day-group" aria-label={formatShortDate(day.date)}>
          <h2 className="section-title">
            {formatShortDate(day.date)}
            {day.date === today && ' · сьогодні'}
          </h2>
          <ul className="workout-list">
            {day.items.map((item) => (
              <WorkoutRow
                key={item.workout.id}
                workout={item.workout}
                heading={`${item.client_name} · ${item.workout.title || 'Без назви'}`}
                back={back}
              />
            ))}
          </ul>
        </section>
      ))}
    </section>
  )
}
