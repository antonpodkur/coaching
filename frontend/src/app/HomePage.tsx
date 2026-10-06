import { useState } from 'react'
import { Link } from 'react-router'

import type { Schemas } from '../api/client'
import { Avatar } from '../shared/Avatar'
import {
  formatKg,
  formatShortDate,
  formatToday,
  localDate,
  parseDate,
  plural,
} from '../shared/format'
import {
  CalendarIcon,
  CheckIcon,
  ChevronIcon,
  PersonIcon,
  PlateIcon,
  ScaleIcon,
} from '../shared/icons'
import { Collapse } from '../shared/Collapse'
import { InstallCard } from '../shared/InstallCard'
import { PushCard } from '../shared/PushCard'
import { Screen } from '../shared/Screen'
import type { NutritionTarget } from '../shared/nutrition'
import { type WeightEntry, formatChange, weeklyAverages, weightTrend } from '../shared/weight'
import { BotMessagesCard } from './BotMessagesCard'
import { useCoachCard } from './coachCard'
import { QuestionnaireCard } from './QuestionnaireCard'
import { questionnaireCardDismissed } from './questionnaire'
import { useMyNutrition } from './nutrition'
import { useMyWeight } from './weight'
import { type ClientWorkoutSummary, useMyWorkouts } from './workouts'

const WEEKDAYS = ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Нд']

/**
 * Today: this week at a glance, the workout to do now or next, the others
 * around it, weight and nutrition, and only then anything the app asks for.
 */
export function HomePage({ client }: { client: Schemas['ClientProfile'] }) {
  const workouts = useMyWorkouts()
  const weight = useMyWeight()
  const nutrition = useMyNutrition()
  const today = localDate(new Date())
  const all = workouts.data ?? []

  // Today's workout, else the next one, else the latest one.
  const main =
    all.find((workout) => workout.date === today) ??
    all.find((workout) => workout.date > today) ??
    all.at(-1)
  const recent = all.filter((workout) => workout !== main && workout.date < today).slice(-2)
  const upcoming = all.filter((workout) => workout !== main && workout.date > today).slice(0, 2)
  const firstName = client.name.split(' ')[0]
  const [botAllowed, setBotAllowed] = useState(client.bot_allowed)
  const [questionnaireLater, setQuestionnaireLater] = useState(questionnaireCardDismissed)

  return (
    <Screen>
      <header className="client-top">
        <div className="client-top-text">
          <p className="muted small">{formatToday()}</p>
          <h1>Привіт, {firstName}</h1>
        </div>
        <Link className="icon-button" to="/app/questionnaire" aria-label="Профіль і анкета">
          {client.avatar ? (
            <Avatar name={client.name} url={client.avatar.url} />
          ) : (
            <PersonIcon />
          )}
        </Link>
      </header>

      <WeekStrip workouts={all} today={today} />

      {workouts.isPending && <p className="muted">Завантаження…</p>}
      {!workouts.data && workouts.fetchStatus === 'paused' && (
        <p className="muted">Немає зв’язку. Тренування з’являться, щойно телефон підключиться.</p>
      )}
      {workouts.data && all.length === 0 && (
        <p className="notice">
          {botAllowed
            ? 'Даша ще не надіслала тренувань. Коли надішле, бот напише тобі.'
            : 'Даша ще не надіслала тренувань. Коли надішле, вони з’являться тут.'}
        </p>
      )}

      {main && <MainCard workout={main} today={today} />}

      {(recent.length > 0 || upcoming.length > 0) && (
        <ul className="list-group" aria-label="Інші тренування">
          {[...recent, ...upcoming].map((workout) => (
            <li key={workout.id}>
              <Link className="list-row" to={`/app/workouts/${workout.id}`}>
                <span
                  className={workout.status === 'done' ? 'list-icon done' : 'list-icon'}
                  aria-hidden="true"
                >
                  {workout.status === 'done' ? <CheckIcon /> : <CalendarIcon />}
                </span>
                <span className="list-text">
                  <span className="list-title">
                    {workout.title || 'Тренування'} · {formatShortDate(workout.date)}
                  </span>
                  <span className="muted small">{statusLine(workout, today)}</span>
                </span>
                <ChevronIcon />
              </Link>
            </li>
          ))}
        </ul>
      )}

      {/* Everything below is drawn with the workouts, so it never pushes them down. */}
      {!workouts.isPending && !weight.isPending && !nutrition.isPending && (
        <>
          <div className="tiles">
            <WeightTile entries={weight.data ?? []} />
            {nutrition.data?.current && <NutritionTile target={nutrition.data.current} />}
          </div>

          {/* One ask at a time: messages first (they bring the reminders), then the questionnaire. */}
          <Collapse show={!botAllowed}>
            <BotMessagesCard onAllowed={() => setBotAllowed(true)} />
          </Collapse>
          <Collapse show={botAllowed && !client.questionnaire_started && !questionnaireLater}>
            <QuestionnaireCard onDismiss={() => setQuestionnaireLater(true)} />
          </Collapse>
          {botAllowed && (client.questionnaire_started || questionnaireLater) && (
            <>
              {/* Inside Telegram the first offers the installed app; outside, the second its notifications. */}
              <InstallCard />
              <PushCard />
            </>
          )}
        </>
      )}
    </Screen>
  )
}

function WeekStrip({ workouts, today }: { workouts: ClientWorkoutSummary[]; today: string }) {
  const now = parseDate(today)
  const monday = new Date(now)
  monday.setDate(now.getDate() - ((now.getDay() + 6) % 7))
  const days = WEEKDAYS.map((label, index) => {
    const day = new Date(monday)
    day.setDate(monday.getDate() + index)
    const date = localDate(day)
    return { label, date, number: day.getDate(), workout: workouts.find((w) => w.date === date) }
  })

  return (
    <ol className="week" aria-label="Цей тиждень">
      {days.map((day) => {
        const className = [
          'week-day',
          day.date === today && 'today',
          day.workout && 'has-workout',
          day.workout?.status === 'done' && 'done',
        ]
          .filter(Boolean)
          .join(' ')
        const content = (
          <>
            <span className="week-label">{day.label}</span>
            <span className="week-number">{day.number}</span>
            <span className="week-dot" aria-hidden="true" />
          </>
        )
        return (
          <li key={day.date}>
            {day.workout ? (
              <Link
                className={className}
                to={`/app/workouts/${day.workout.id}`}
                aria-label={`${day.label}, ${day.number}: ${day.workout.title || 'тренування'}`}
              >
                {content}
              </Link>
            ) : (
              <span className={className}>{content}</span>
            )}
          </li>
        )
      })}
    </ol>
  )
}

function MainCard({ workout, today }: { workout: ClientWorkoutSummary; today: string }) {
  const when =
    workout.date === today
      ? 'Сьогодні'
      : workout.date > today
        ? `Наступне · ${formatShortDate(workout.date)}`
        : formatShortDate(workout.date)
  const coach = useCoachCard()
  const started = workout.done_set_count > 0
  const allDone = workout.set_count > 0 && workout.done_set_count === workout.set_count
  return (
    <section className="today-card" aria-label="Тренування">
      {workout.thumbnail_url && <img className="today-image" src={workout.thumbnail_url} alt="" />}
      <div className="today-body">
        <div className="today-top">
          <span className="today-when">{when}</span>
          {coach && (
            <span className="today-from">
              <Avatar name={coach.name} url={coach.avatar_url} small />
              {coach.name}
            </span>
          )}
        </div>
        <h2>{workout.title || 'Тренування'}</h2>
        <span className="muted small">
          {workout.exercise_count} {plural(workout.exercise_count, 'вправа', 'вправи', 'вправ')} ·{' '}
          {workout.set_count} {plural(workout.set_count, 'підхід', 'підходи', 'підходів')}
          {started && workout.status !== 'done' && ` · виконано ${workout.done_set_count}`}
        </span>
        {workout.status !== 'done' && allDone ? (
          <Link className="button primary block" to={`/app/workouts/${workout.id}/finish`}>
            Завершити тренування
            <ChevronIcon />
          </Link>
        ) : (
          <Link className="button primary block" to={`/app/workouts/${workout.id}`}>
            {workout.status === 'done'
              ? 'Переглянути'
              : started
                ? 'Продовжити тренування'
                : 'Відкрити тренування'}
            <ChevronIcon />
          </Link>
        )}
        {workout.status === 'done' && <span className="done-line">Виконано, звіт надіслано</span>}
      </div>
    </section>
  )
}

function statusLine(workout: ClientWorkoutSummary, today: string): string {
  if (workout.status === 'done') return 'Виконано, звіт надіслано'
  if (workout.date > today) return 'Заплановано'
  return workout.done_set_count > 0
    ? `Виконано ${workout.done_set_count} з ${workout.set_count}`
    : 'Ще не почато'
}

/** Weight at a glance: the latest weigh-in, how the trend moved, and its line. */
function WeightTile({ entries }: { entries: WeightEntry[] }) {
  const trend = weightTrend(entries)
  const change = trend && formatChange(trend)
  return (
    <Link className="tile" to="/app/weight">
      <span className="tile-label">
        <ScaleIcon />
        Вага
      </span>
      {trend ? (
        <>
          <span className="tile-value">
            {formatKg(trend.latest.kg)} <small>кг</small>
          </span>
          <span className="muted small">{change ?? 'Перший запис'}</span>
          <Sparkline entries={entries} />
        </>
      ) : (
        <span className="muted small">Записати вагу</span>
      )}
    </Link>
  )
}

/** The weekly average over the last month, as a small line. */
function Sparkline({ entries }: { entries: WeightEntry[] }) {
  const points = weeklyAverages(entries).slice(-30)
  if (points.length < 2) return null
  const kgs = points.map((point) => point.kg)
  const low = Math.min(...kgs)
  const span = Math.max(Math.max(...kgs) - low, 1)
  const path = points
    .map((point, index) => {
      const x = (index / (points.length - 1)) * 100
      const y = 26 - ((point.kg - low) / span) * 22
      return `${x.toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')
  return (
    <svg className="sparkline" viewBox="0 0 100 28" preserveAspectRatio="none" aria-hidden="true">
      <polyline points={path} />
    </svg>
  )
}

/** The daily target from the coach: calories, with the grams under them. */
function NutritionTile({ target }: { target: NutritionTarget }) {
  return (
    <Link className="tile" to="/app/nutrition">
      <span className="tile-label">
        <PlateIcon />
        Харчування
      </span>
      <span className="tile-value">
        {target.kcal} <small>ккал</small>
      </span>
      <span className="muted small">
        Б {target.protein_g} · Ж {target.fat_g} · В {target.carbs_g}
      </span>
    </Link>
  )
}
