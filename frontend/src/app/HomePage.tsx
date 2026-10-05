import { useState } from 'react'
import { Link } from 'react-router'

import type { Schemas } from '../api/client'
import { formatShortDate, formatToday, localDate, parseDate, plural } from '../shared/format'
import { CalendarIcon, CheckIcon, ChevronIcon, PersonIcon } from '../shared/icons'
import { HomeScreenCard } from '../shared/HomeScreenCard'
import { Screen } from '../shared/Screen'
import { BotMessagesCard } from './BotMessagesCard'
import { QuestionnaireCard } from './QuestionnaireCard'
import { questionnaireCardDismissed } from './questionnaire'
import { type ClientWorkoutSummary, useMyWorkouts } from './workouts'

const WEEKDAYS = ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Нд']

/** Today: this week at a glance, and the workout to do now or next. */
export function HomePage({ client }: { client: Schemas['ClientProfile'] }) {
  const workouts = useMyWorkouts()
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
        <Link className="icon-button" to="/app/questionnaire" aria-label="Анкета для Даші">
          <PersonIcon />
        </Link>
      </header>

      {/* One ask at a time: messages first (they bring the reminders), then the questionnaire. */}
      {!botAllowed ? (
        <BotMessagesCard onAllowed={() => setBotAllowed(true)} />
      ) : !client.questionnaire_started && !questionnaireLater ? (
        <QuestionnaireCard onDismiss={() => setQuestionnaireLater(true)} />
      ) : (
        <HomeScreenCard />
      )}

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
        <ul className="other-workouts">
          {[...recent, ...upcoming].map((workout) => (
            <li key={workout.id}>
              <Link className="other-workout" to={`/app/workouts/${workout.id}`}>
                <span className={workout.status === 'done' ? 'round-icon done' : 'round-icon'}>
                  {workout.status === 'done' ? <CheckIcon /> : <CalendarIcon />}
                </span>
                <span className="other-text">
                  <span className="other-title">
                    {workout.title || 'Тренування'} · {formatShortDate(workout.date)}
                  </span>
                  <span className="muted small">{statusLine(workout, today)}</span>
                </span>
              </Link>
            </li>
          ))}
        </ul>
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
  const started = workout.done_set_count > 0
  return (
    <section className="today-card" aria-label="Тренування">
      {workout.thumbnail_url && <img className="today-image" src={workout.thumbnail_url} alt="" />}
      <div className="today-body">
        <span className="today-when">{when}</span>
        <h2>{workout.title || 'Тренування'}</h2>
        <span className="muted small">
          {workout.exercise_count} {plural(workout.exercise_count, 'вправа', 'вправи', 'вправ')} ·{' '}
          {workout.set_count} {plural(workout.set_count, 'підхід', 'підходи', 'підходів')}
          {started && workout.status !== 'done' && ` · виконано ${workout.done_set_count}`}
        </span>
        <Link className="button primary block" to={`/app/workouts/${workout.id}`}>
          {workout.status === 'done'
            ? 'Переглянути'
            : started
              ? 'Продовжити тренування'
              : 'Відкрити тренування'}
          <ChevronIcon />
        </Link>
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
