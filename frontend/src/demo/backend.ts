import type { Schemas } from '../api/client'

/**
 * The demo's backend: the real API's answers, worked out in the browser from
 * made-up data (src/demo/seed.ts). It follows backend/src closely enough for
 * the app to behave as it does with a real coach and client: what counts as
 * "differs from the plan", "Минулого разу", the next workout, unseen reports.
 * Uploads and Telegram are not part of the demo and answer as not set up.
 */

type WorkoutStatus = Schemas['WorkoutStatus']

export interface DemoSet {
  id: string
  target_kg: number | null
  target_reps_min: number
  target_reps_max: number
  actual_kg: number | null
  actual_reps: number | null
  completed_at: string | null
  client_updated_at: string | null
}

export interface DemoRow {
  id: string
  exercise_id: string
  per_side_label: string | null
  note: string | null
  sets: DemoSet[]
}

export interface DemoReport {
  effort: Schemas['Effort']
  comment: string
  duration_min: number | null
  finished_at: string
  seen_at: string | null
}

export interface DemoWorkout {
  id: string
  client_id: string
  title: string
  date: string | null
  status: WorkoutStatus
  version: number
  published_at: string | null
  opened_at: string | null
  created_at: string
  rows: DemoRow[]
  report: DemoReport | null
}

export interface DemoClient {
  id: string
  name: string
  created_at: string
  paid_until: string | null
  archived_at: string | null
  joined: boolean
  invite_expires_at: string | null
  birth_year: number | null
  height_cm: number | null
  sex: Schemas['Sex'] | null
  weights: Schemas['WeightEntry'][]
  /** Newest first. */
  targets: Schemas['NutritionTarget'][]
}

export interface DemoExercise {
  id: string
  name: string
  measure: Schemas['Measure']
  muscle_group: string | null
  description: string | null
  in_library: boolean
  archived: boolean
}

export interface DemoState {
  coach: { id: string; name: string; timezone: string }
  /** The client whose phone the demo shows. */
  clientId: string
  /** Today's draft the landing page's steps follow: publish, log, finish, read. */
  featured: string
  clients: DemoClient[]
  exercises: DemoExercise[]
  workouts: DemoWorkout[]
}

export interface DemoRequest {
  method: string
  path: string
  query: Record<string, string>
  body: unknown
  ifMatch: string | null
  role: 'coach' | 'client'
}

export interface DemoResponse {
  status: number
  body?: unknown
}

// ── Dates ────────────────────────────────────────────────────────────────

const pad = (n: number) => String(n).padStart(2, '0')

/** The phone's calendar date, `offset` days from today: `2026-10-12`. */
export function localDay(offset = 0): string {
  const date = new Date()
  date.setDate(date.getDate() + offset)
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

/** A moment on the day `offset` days from today, at `hour:minute` on the phone's clock. */
export function localTime(offset: number, hour: number, minute = 0): string {
  const date = new Date()
  date.setDate(date.getDate() + offset)
  date.setHours(hour, minute, 0, 0)
  return date.toISOString()
}

const nowIso = () => new Date().toISOString()

// ── Answers ──────────────────────────────────────────────────────────────

class Failure {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string) {
    this.status = status
    this.code = code
  }
}

function fail(status: number, code: string): never {
  throw new Failure(status, code)
}

const ok = (body?: unknown, status = body === undefined ? 204 : 200): DemoResponse => ({
  status,
  body,
})

// ── What the real backend works out ──────────────────────────────────────

/** backend/migrations: `set_differs`. */
function differs(set: DemoSet): boolean {
  if (set.completed_at === null) return false
  return (
    set.actual_kg !== set.target_kg ||
    set.actual_reps === null ||
    set.actual_reps < set.target_reps_min ||
    set.actual_reps > set.target_reps_max
  )
}

/** backend/src/history.rs: what the client did the last time each exercise came up. */
function lastTime(
  state: DemoState,
  clientId: string,
  workoutId: string,
  before: string | null,
  exerciseId: string,
): Schemas['PastSet'][] {
  const earlier = state.workouts
    .filter(
      (workout) =>
        workout.client_id === clientId &&
        workout.id !== workoutId &&
        workout.date !== null &&
        (before === null || workout.date <= before),
    )
    .sort(
      (a, b) =>
        (b.date ?? '').localeCompare(a.date ?? '') || b.created_at.localeCompare(a.created_at),
    )
  for (const workout of earlier) {
    for (const row of workout.rows) {
      if (row.exercise_id !== exerciseId) continue
      const done = row.sets.filter((set) => set.completed_at !== null)
      if (done.length > 0) return done.map((set) => ({ kg: set.actual_kg, reps: set.actual_reps }))
    }
  }
  return []
}

function exerciseOf(state: DemoState, id: string): DemoExercise {
  return state.exercises.find((exercise) => exercise.id === id) ?? fail(404, 'not_found')
}

function clientOf(state: DemoState, id: string): DemoClient {
  return state.clients.find((client) => client.id === id) ?? fail(404, 'not_found')
}

function workoutOf(state: DemoState, id: string): DemoWorkout {
  return state.workouts.find((workout) => workout.id === id) ?? fail(404, 'not_found')
}

const setsOf = (workout: DemoWorkout) => workout.rows.flatMap((row) => row.sets)

function summary(workout: DemoWorkout): Schemas['WorkoutSummary'] {
  const sets = setsOf(workout)
  return {
    id: workout.id,
    title: workout.title,
    date: workout.date,
    status: workout.status,
    published_at: workout.published_at,
    opened: workout.opened_at !== null,
    exercise_count: workout.rows.length,
    set_count: sets.length,
    done_set_count: sets.filter((set) => set.completed_at !== null).length,
    different_count: sets.filter(differs).length,
    new_videos: 0,
    report: workout.report && {
      effort: workout.report.effort,
      finished_at: workout.report.finished_at,
      has_comment: workout.report.comment !== '',
      seen: workout.report.seen_at !== null,
    },
  }
}

function coachClient(state: DemoState, client: DemoClient): Schemas['CoachClient'] {
  const theirs = state.workouts.filter((workout) => workout.client_id === client.id)
  const today = localDay()
  const upcoming = theirs
    .filter((workout) => workout.status === 'published' && workout.date !== null && workout.date >= today)
    .map((workout) => workout.date as string)
    .sort()
  return {
    id: client.id,
    name: client.name,
    created_at: client.created_at,
    archived: client.archived_at !== null,
    avatar_url: null,
    birth_year: client.birth_year,
    height_cm: client.height_cm,
    sex: client.sex,
    joined: client.joined,
    invite_expires_at: client.invite_expires_at,
    paid_until: client.paid_until,
    drafts: theirs.filter((workout) => workout.status === 'draft').length,
    unseen_reports: theirs.filter((workout) => workout.report && !workout.report.seen_at).length,
    next_workout_on: upcoming[0] ?? null,
    new_videos: 0,
    gym_photos: 0,
    gym_videos: 0,
  }
}

function workoutDoc(state: DemoState, workout: DemoWorkout): Schemas['Workout'] {
  return {
    id: workout.id,
    client_id: workout.client_id,
    title: workout.title,
    date: workout.date,
    status: workout.status,
    version: workout.version,
    published_at: workout.published_at,
    exercises: workout.rows.map((row) => {
      const exercise = exerciseOf(state, row.exercise_id)
      return {
        id: row.id,
        exercise_id: row.exercise_id,
        name: exercise.name,
        measure: exercise.measure,
        in_library: exercise.in_library,
        per_side_label: row.per_side_label,
        note: row.note,
        thumbnail_url: null,
        last_time: lastTime(state, workout.client_id, workout.id, workout.date, row.exercise_id),
        sets: row.sets.map((set) => ({
          id: set.id,
          kg: set.target_kg,
          reps_min: set.target_reps_min,
          reps_max: set.target_reps_max,
        })),
      }
    }),
  }
}

function exerciseBody(exercise: DemoExercise): Schemas['Exercise'] {
  return {
    id: exercise.id,
    name: exercise.name,
    aliases: [],
    description: exercise.description,
    in_library: exercise.in_library,
    measure: exercise.measure,
    muscle_group: exercise.muscle_group,
    photos: [],
    upload: null,
    video: null,
  }
}

function inviteLink(client: DemoClient): Schemas['InviteLink'] {
  const expires = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString()
  client.invite_expires_at = expires
  return { url: 'https://getcoachin.app/', expires_at: expires, prepared_message_id: null }
}

export function coachProfile(state: DemoState): Schemas['CoachProfile'] {
  return {
    id: state.coach.id,
    name: state.coach.name,
    timezone: state.coach.timezone,
    avatar_url: null,
    app_installed: true,
  }
}

export function clientProfile(state: DemoState): Schemas['ClientProfile'] {
  const client = clientOf(state, state.clientId)
  return {
    id: client.id,
    name: client.name,
    coach: { name: state.coach.name, avatar_url: null },
    app_installed: true,
    bot_allowed: true,
    questionnaire_started: true,
    avatar: null,
    timezone: state.coach.timezone,
  }
}

// ── Reading request bodies ───────────────────────────────────────────────

type Body = Record<string, unknown>

function bodyOf(request: DemoRequest): Body {
  return typeof request.body === 'object' && request.body !== null ? (request.body as Body) : {}
}

const text = (value: unknown) => (typeof value === 'string' ? value : null)
const num = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) ? value : null)

function newId(): string {
  return crypto.randomUUID()
}

// ── Routes ───────────────────────────────────────────────────────────────

type Handler = (state: DemoState, request: DemoRequest, params: string[]) => DemoResponse

interface Route {
  method: string
  pattern: RegExp
  /** Changes the data, so the other phone refreshes. */
  writes: boolean
  handler: Handler
}

const routes: Route[] = []

function route(method: string, path: string, handler: Handler) {
  const pattern = new RegExp(`^${path.replace(/\{[^}]+\}/g, '([^/]+)')}$`)
  routes.push({ method, pattern, writes: method !== 'GET', handler })
}

/** Answers one request against `state`, changing it as the real backend would. */
export function handle(
  state: DemoState,
  request: DemoRequest,
): { response: DemoResponse; changed: boolean } {
  for (const candidate of routes) {
    if (candidate.method !== request.method) continue
    const match = candidate.pattern.exec(request.path)
    if (!match) continue
    try {
      const response = candidate.handler(state, request, match.slice(1).map(decodeURIComponent))
      return { response, changed: candidate.writes || request.path.startsWith('/workouts/') }
    } catch (err) {
      if (err instanceof Failure) {
        return { response: { status: err.status, body: { error: err.code } }, changed: false }
      }
      throw err
    }
  }
  return { response: { status: 404, body: { error: 'not_found' } }, changed: false }
}

// Signing in, notifications and uploads have nothing to do in the demo.

route('POST', '/auth/refresh', (state, request) =>
  ok(
    request.role === 'coach'
      ? { role: 'coach', token: 'demo', coach: coachProfile(state) }
      : { role: 'client', token: 'demo', client: clientProfile(state) },
  ),
)
route('POST', '/auth/installed', () => ok())
route('GET', '/push/key', () => fail(503, 'push_not_configured'))
route('PUT', '/push/subscription', () => ok())
route('DELETE', '/push/subscription', () => ok())
route('PUT', '/coach/me/timezone', () => ok())
route('PUT', '/coach/me/avatar', () => fail(503, 'photos_not_configured'))
route('DELETE', '/coach/me/avatar', () => ok())
route('POST', '/coach/exercises/{id}/photos', () => fail(503, 'photos_not_configured'))
route('DELETE', '/coach/exercise-photos/{id}', () => ok())
route('POST', '/coach/exercises/{id}/video-upload', () => fail(503, 'video_not_configured'))
route('POST', '/coach/import/parse', () => fail(503, 'import_not_in_demo'))
route('PUT', '/me/timezone', () => ok())
route('POST', '/me/bot-allowed', () => ok())
route('PUT', '/me/avatar', () => fail(503, 'photos_not_configured'))
route('DELETE', '/me/avatar', () => ok())
route('POST', '/me/gym/photos', () => fail(503, 'photos_not_configured'))
route('POST', '/me/gym/videos', () => fail(503, 'client_videos_not_configured'))
route('POST', '/workout-exercises/{id}/videos', () => fail(503, 'client_videos_not_configured'))

// ── Coach: clients ───────────────────────────────────────────────────────

route('GET', '/coach/clients', (state, request) => {
  const archived = request.query.archived === 'true'
  return ok(
    state.clients
      .filter((client) => (client.archived_at !== null) === archived)
      .sort(
        (a, b) =>
          (b.archived_at ?? '').localeCompare(a.archived_at ?? '') ||
          b.created_at.localeCompare(a.created_at),
      )
      .map((client) => coachClient(state, client)),
  )
})

route('POST', '/coach/clients', (state, request) => {
  const name = text(bodyOf(request).name)?.trim() ?? ''
  if (!name || name.length > 80) fail(400, 'invalid_name')
  const client: DemoClient = {
    id: newId(),
    name,
    created_at: nowIso(),
    paid_until: null,
    archived_at: null,
    joined: false,
    invite_expires_at: null,
    birth_year: null,
    height_cm: null,
    sex: null,
    weights: [],
    targets: [],
  }
  state.clients.push(client)
  const invite = inviteLink(client)
  return ok({ client: coachClient(state, client), invite }, 201)
})

route('GET', '/coach/clients/{id}', (state, _request, [id = '']) =>
  ok(coachClient(state, clientOf(state, id))),
)

route('PATCH', '/coach/clients/{id}', (state, request, [id = '']) => {
  const client = clientOf(state, id)
  const changes = bodyOf(request)
  const name = text(changes.name)
  if (name !== null) {
    if (!name.trim() || name.trim().length > 80) fail(400, 'invalid_name')
    client.name = name.trim()
  }
  if ('paid_until' in changes) client.paid_until = text(changes.paid_until)
  if (changes.archived === true) client.archived_at = nowIso()
  if (changes.archived === false) client.archived_at = null
  return ok(coachClient(state, client))
})

route('POST', '/coach/clients/{id}/invite', (state, _request, [id = '']) =>
  ok(inviteLink(clientOf(state, id))),
)

route('GET', '/coach/clients/{id}/nutrition', (state, _request, [id = '']) =>
  ok({ targets: clientOf(state, id).targets }),
)

route('POST', '/coach/clients/{id}/nutrition', (state, request, [id = '']) => {
  const client = clientOf(state, id)
  const body = bodyOf(request)
  const grams = [num(body.protein_g), num(body.fat_g), num(body.carbs_g)]
  if (grams.some((g) => g === null || g < 0 || g > 1000)) fail(400, 'invalid_grams')
  const [protein_g = 0, fat_g = 0, carbs_g = 0] = grams as number[]
  const target: Schemas['NutritionTarget'] = {
    id: newId(),
    protein_g,
    fat_g,
    carbs_g,
    kcal: 4 * protein_g + 4 * carbs_g + 9 * fat_g,
    note: text(body.note)?.trim() || null,
    set_at: nowIso(),
  }
  client.targets.unshift(target)
  return ok({ client_notified: true, target })
})

route('GET', '/coach/clients/{id}/questionnaire', (state, _request, [id = '']) => {
  const client = clientOf(state, id)
  return ok({
    birth_year: client.birth_year,
    height_cm: client.height_cm,
    sex: client.sex,
    photos: [],
    videos: [],
  })
})

route('GET', '/coach/clients/{id}/weight', (state, _request, [id = '']) =>
  ok([...clientOf(state, id).weights].sort((a, b) => a.date.localeCompare(b.date))),
)

route('GET', '/coach/clients/{id}/workouts', (state, _request, [id = '']) => {
  clientOf(state, id)
  return ok(
    state.workouts
      .filter((workout) => workout.client_id === id)
      .sort(
        (a, b) =>
          (a.date === null ? -1 : 0) - (b.date === null ? -1 : 0) ||
          (b.date ?? '').localeCompare(a.date ?? '') ||
          b.created_at.localeCompare(a.created_at),
      )
      .map(summary),
  )
})

// ── Coach: exercises ─────────────────────────────────────────────────────

function nameTaken(state: DemoState, name: string, except?: string) {
  const lower = name.toLocaleLowerCase('uk')
  return state.exercises.some(
    (exercise) =>
      exercise.id !== except && !exercise.archived && exercise.name.toLocaleLowerCase('uk') === lower,
  )
}

route('GET', '/coach/exercises', (state) =>
  ok(
    state.exercises
      .filter((exercise) => exercise.in_library && !exercise.archived)
      .sort((a, b) => a.name.localeCompare(b.name, 'uk'))
      .map(exerciseBody),
  ),
)

route('POST', '/coach/exercises', (state, request) => {
  const body = bodyOf(request)
  const name = text(body.name)?.trim() ?? ''
  if (!name || name.length > 120) fail(400, 'invalid_name')
  if (nameTaken(state, name)) fail(409, 'name_taken')
  const exercise: DemoExercise = {
    id: newId(),
    name,
    measure: (text(body.measure) as Schemas['Measure'] | null) ?? 'weight',
    muscle_group: text(body.muscle_group)?.trim() || null,
    description: null,
    in_library: body.in_library !== false,
    archived: false,
  }
  state.exercises.push(exercise)
  return ok(exerciseBody(exercise), 201)
})

route('GET', '/coach/exercises/{id}', (state, _request, [id = '']) =>
  ok(exerciseBody(exerciseOf(state, id))),
)

route('PATCH', '/coach/exercises/{id}', (state, request, [id = '']) => {
  const exercise = exerciseOf(state, id)
  const changes = bodyOf(request)
  const name = text(changes.name)
  if (name !== null) {
    if (!name.trim() || name.trim().length > 120) fail(400, 'invalid_name')
    if (nameTaken(state, name.trim(), id)) fail(409, 'name_taken')
    exercise.name = name.trim()
  }
  const measure = text(changes.measure) as Schemas['Measure'] | null
  if (measure && measure !== exercise.measure) {
    const used = state.workouts.some((workout) =>
      workout.rows.some((row) => row.exercise_id === id && row.sets.length > 0),
    )
    if (used && (measure === 'time' || exercise.measure === 'time')) fail(409, 'measure_in_use')
    exercise.measure = measure
  }
  if ('muscle_group' in changes) exercise.muscle_group = text(changes.muscle_group)?.trim() || null
  if ('description' in changes) exercise.description = text(changes.description)?.trim() || null
  if (changes.in_library === true) exercise.in_library = true
  if (changes.archived === true) exercise.archived = true
  if (changes.archived === false) exercise.archived = false
  return ok(exerciseBody(exercise))
})

route('POST', '/coach/exercises/{id}/video-uploaded', (state, _request, [id = '']) =>
  ok(exerciseBody(exerciseOf(state, id))),
)

// ── Coach: workouts ──────────────────────────────────────────────────────

route('GET', '/coach/workouts', (state, request) => {
  const from = request.query.from ?? ''
  const to = request.query.to ?? ''
  const days = (Date.parse(to) - Date.parse(from)) / 86_400_000
  if (!(days >= 0 && days <= 62)) fail(400, 'invalid_range')
  const active = new Map(
    state.clients.filter((client) => client.archived_at === null).map((c) => [c.id, c.name]),
  )
  return ok(
    state.workouts
      .filter(
        (workout) =>
          active.has(workout.client_id) &&
          (!request.query.client_id || workout.client_id === request.query.client_id) &&
          (workout.date === null
            ? workout.status === 'draft'
            : workout.date >= from && workout.date <= to),
      )
      .sort(
        (a, b) =>
          (b.date === null ? 1 : 0) - (a.date === null ? 1 : 0) ||
          (a.date ?? '').localeCompare(b.date ?? '') ||
          a.created_at.localeCompare(b.created_at),
      )
      .map((workout) => ({
        client_id: workout.client_id,
        client_name: active.get(workout.client_id) ?? '',
        workout: summary(workout),
      })),
  )
})

const COPY_DAYS_LATER = 7

function addDays(date: string, days: number): string {
  const [year = 0, month = 1, day = 1] = date.split('-').map(Number)
  const next = new Date(year, month - 1, day + days)
  return `${next.getFullYear()}-${pad(next.getMonth() + 1)}-${pad(next.getDate())}`
}

route('POST', '/coach/workouts', (state, request) => {
  const body = bodyOf(request)
  const client = clientOf(state, text(body.client_id) ?? '')
  if (client.archived_at) fail(404, 'not_found')
  const source = text(body.copy_from) ? workoutOf(state, text(body.copy_from) ?? '') : null
  const date = text(body.date) ?? (source?.date ? addDays(source.date, COPY_DAYS_LATER) : null)
  const workout: DemoWorkout = {
    id: newId(),
    client_id: client.id,
    title: source?.title ?? '',
    date,
    status: 'draft',
    version: 1,
    published_at: null,
    opened_at: null,
    created_at: nowIso(),
    report: null,
    rows: (source?.rows ?? []).map((row) => ({
      id: newId(),
      exercise_id: row.exercise_id,
      per_side_label: row.per_side_label,
      note: row.note,
      sets: row.sets.map((set) => ({
        id: newId(),
        target_kg: set.target_kg,
        target_reps_min: set.target_reps_min,
        target_reps_max: set.target_reps_max,
        actual_kg: null,
        actual_reps: null,
        completed_at: null,
        client_updated_at: null,
      })),
    })),
  }
  state.workouts.push(workout)
  return ok(workoutDoc(state, workout), 201)
})

route('GET', '/coach/workouts/{id}', (state, _request, [id = '']) =>
  ok(workoutDoc(state, workoutOf(state, id))),
)

route('PUT', '/coach/workouts/{id}', (state, request, [id = '']) => {
  const workout = workoutOf(state, id)
  const version = Number(request.ifMatch?.replace(/\D/g, ''))
  if (!request.ifMatch || !Number.isInteger(version)) fail(400, 'version_required')
  if (version !== workout.version) fail(409, 'version_conflict')

  const body = bodyOf(request)
  const title = text(body.title)?.trim() ?? ''
  if (title.length > 120) fail(400, 'invalid_title')
  const rows = Array.isArray(body.exercises) ? (body.exercises as Body[]) : []
  const ownSets = new Map(setsOf(workout).map((set) => [set.id, set]))
  const others = new Set(
    state.workouts
      .filter((other) => other.id !== id)
      .flatMap((other) => [...other.rows.map((row) => row.id), ...setsOf(other).map((s) => s.id)]),
  )

  workout.rows = rows.map((row) => {
    const rowId = text(row.id) ?? fail(400, 'invalid_ids')
    if (others.has(rowId)) fail(400, 'invalid_ids')
    const exerciseId = text(row.exercise_id) ?? ''
    if (!state.exercises.some((exercise) => exercise.id === exerciseId)) {
      fail(400, 'unknown_exercise')
    }
    const sets = Array.isArray(row.sets) ? (row.sets as Body[]) : []
    return {
      id: rowId,
      exercise_id: exerciseId,
      per_side_label: text(row.per_side_label)?.trim() || null,
      note: text(row.note)?.trim() || null,
      sets: sets.map((set) => {
        const setId = text(set.id) ?? fail(400, 'invalid_ids')
        if (others.has(setId)) fail(400, 'invalid_ids')
        const min = num(set.reps_min)
        const max = num(set.reps_max)
        if (min === null || max === null || min < 0 || max < min) fail(400, 'invalid_set')
        const kept = ownSets.get(setId)
        return {
          id: setId,
          target_kg: num(set.kg),
          target_reps_min: min as number,
          target_reps_max: max as number,
          actual_kg: kept?.actual_kg ?? null,
          actual_reps: kept?.actual_reps ?? null,
          completed_at: kept?.completed_at ?? null,
          client_updated_at: kept?.client_updated_at ?? null,
        }
      }),
    }
  })
  workout.title = title
  workout.date = text(body.date)
  workout.version += 1
  return ok(workoutDoc(state, workout))
})

route('DELETE', '/coach/workouts/{id}', (state, _request, [id = '']) => {
  workoutOf(state, id)
  state.workouts = state.workouts.filter((workout) => workout.id !== id)
  return ok()
})

route('POST', '/coach/workouts/{id}/publish', (state, _request, [id = '']) => {
  const workout = workoutOf(state, id)
  if (!workout.date) fail(400, 'date_required')
  if (workout.rows.length === 0) fail(400, 'empty_workout')
  if (workout.rows.some((row) => row.sets.length === 0)) fail(400, 'exercise_without_sets')
  if (workout.status === 'draft') {
    workout.status = 'published'
    workout.published_at = nowIso()
  }
  return ok({ client_notified: true, workout: workoutDoc(state, workout) })
})

route('GET', '/coach/workouts/{id}/results', (state, _request, [id = '']) => {
  const workout = workoutOf(state, id)
  const report = workout.report
  return ok({
    id: workout.id,
    client_id: workout.client_id,
    title: workout.title,
    date: workout.date,
    status: workout.status,
    report: report && {
      comment: report.comment,
      duration_min: report.duration_min,
      effort: report.effort,
      finished_at: report.finished_at,
      seen: report.seen_at !== null,
    },
    exercises: workout.rows.map((row) => {
      const exercise = exerciseOf(state, row.exercise_id)
      return {
        id: row.id,
        name: exercise.name,
        measure: exercise.measure,
        per_side_label: row.per_side_label,
        videos: [],
        sets: row.sets.map((set) => ({
          id: set.id,
          target_kg: set.target_kg,
          target_reps_min: set.target_reps_min,
          target_reps_max: set.target_reps_max,
          actual_kg: set.actual_kg,
          actual_reps: set.actual_reps,
          completed: set.completed_at !== null,
          differs: differs(set),
        })),
      }
    }),
  } satisfies Schemas['WorkoutResults'])
})

route('POST', '/coach/workouts/{id}/report/seen', (state, _request, [id = '']) => {
  const workout = workoutOf(state, id)
  if (workout.report && !workout.report.seen_at) workout.report.seen_at = nowIso()
  return ok()
})

// ── Client ───────────────────────────────────────────────────────────────

/** The demo client's workout, if they may see it: published or done. */
function myWorkout(state: DemoState, id: string): DemoWorkout {
  const workout = workoutOf(state, id)
  if (workout.client_id !== state.clientId || workout.status === 'draft') fail(404, 'not_found')
  return workout
}

route('GET', '/me', (state) => ok(clientProfile(state)))

route('GET', '/me/workouts', (state) =>
  ok(
    state.workouts
      .filter(
        (workout) =>
          workout.client_id === state.clientId && workout.status !== 'draft' && workout.date,
      )
      .sort(
        (a, b) =>
          (a.date ?? '').localeCompare(b.date ?? '') || a.created_at.localeCompare(b.created_at),
      )
      .map((workout): Schemas['ClientWorkoutSummary'] => {
        const sets = setsOf(workout)
        return {
          id: workout.id,
          title: workout.title,
          date: workout.date as string,
          status: workout.status,
          exercise_count: workout.rows.length,
          set_count: sets.length,
          done_set_count: sets.filter((set) => set.completed_at !== null).length,
          thumbnail_url: null,
        }
      }),
  ),
)

route('GET', '/workouts/{id}', (state, _request, [id = '']) => {
  const workout = myWorkout(state, id)
  // Opening it is what the coach sees as "відкрито".
  workout.opened_at ??= nowIso()
  const report = workout.report
  return ok({
    id: workout.id,
    title: workout.title,
    date: workout.date as string,
    status: workout.status,
    videos_enabled: false,
    report: report && {
      comment: report.comment,
      duration_min: report.duration_min,
      effort: report.effort,
      finished_at: report.finished_at,
    },
    exercises: workout.rows.map((row) => {
      const exercise = exerciseOf(state, row.exercise_id)
      return {
        id: row.id,
        name: exercise.name,
        measure: exercise.measure,
        description: exercise.description,
        note: row.note,
        per_side_label: row.per_side_label,
        photos: [],
        video: null,
        videos: [],
        last_time: lastTime(state, workout.client_id, workout.id, workout.date, row.exercise_id),
        sets: row.sets.map((set) => ({
          id: set.id,
          target_kg: set.target_kg,
          target_reps_min: set.target_reps_min,
          target_reps_max: set.target_reps_max,
          actual_kg: set.actual_kg,
          actual_reps: set.actual_reps,
          completed: set.completed_at !== null,
          client_updated_at: set.client_updated_at,
        })),
      }
    }),
  } satisfies Schemas['ClientWorkout'])
})

route('PUT', '/sets/{id}/result', (state, request, [id = '']) => {
  const body = bodyOf(request)
  const kg = body.actual_kg === null || body.actual_kg === undefined ? null : num(body.actual_kg)
  const reps =
    body.actual_reps === null || body.actual_reps === undefined ? null : num(body.actual_reps)
  if ((kg !== null && (kg <= 0 || kg > 1000)) || (reps !== null && (reps < 0 || reps > 3600))) {
    fail(400, 'invalid_result')
  }
  const workout = state.workouts.find(
    (candidate) =>
      candidate.client_id === state.clientId &&
      candidate.status !== 'draft' &&
      setsOf(candidate).some((set) => set.id === id),
  )
  const set = workout ? setsOf(workout).find((candidate) => candidate.id === id) : undefined
  if (!set) return fail(404, 'not_found')
  const updatedAt = text(body.client_updated_at) ?? nowIso()
  if (set.client_updated_at && set.client_updated_at > updatedAt) return ok()
  set.actual_kg = kg
  set.actual_reps = reps
  set.completed_at = body.completed === true ? (set.completed_at ?? nowIso()) : null
  set.client_updated_at = updatedAt
  return ok()
})

route('POST', '/workouts/{id}/finish', (state, request, [id = '']) => {
  const workout = myWorkout(state, id)
  const body = bodyOf(request)
  const effort = text(body.effort) as Schemas['Effort'] | null
  const comment = text(body.comment)?.trim() ?? ''
  if (!effort || comment.length > 2000) fail(400, 'invalid_report')
  workout.status = 'done'
  workout.report = {
    effort: effort as Schemas['Effort'],
    comment,
    duration_min: num(body.duration_min),
    finished_at: workout.report?.finished_at ?? nowIso(),
    seen_at: workout.report?.seen_at ?? null,
  }
  return ok()
})

route('GET', '/me/nutrition', (state) =>
  ok({ current: clientOf(state, state.clientId).targets[0] ?? null }),
)

route('GET', '/me/questionnaire', (state) => {
  const client = clientOf(state, state.clientId)
  return ok({
    birth_year: client.birth_year,
    height_cm: client.height_cm,
    sex: client.sex,
    photos: [],
    videos: [],
  })
})

route('PUT', '/me/questionnaire', (state, request) => {
  const client = clientOf(state, state.clientId)
  const answers = bodyOf(request)
  client.birth_year = num(answers.birth_year)
  client.height_cm = num(answers.height_cm)
  client.sex = (text(answers.sex) as Schemas['Sex'] | null) ?? null
  return ok({
    birth_year: client.birth_year,
    height_cm: client.height_cm,
    sex: client.sex,
    photos: [],
    videos: [],
  })
})

route('GET', '/me/weight', (state) =>
  ok([...clientOf(state, state.clientId).weights].sort((a, b) => a.date.localeCompare(b.date))),
)

route('PUT', '/me/weight/{date}', (state, request, [date = '']) => {
  const kg = num(bodyOf(request).kg)
  if (kg === null || kg < 20 || kg > 400) fail(400, 'invalid_weight')
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) fail(400, 'invalid_date')
  const client = clientOf(state, state.clientId)
  const entry = { date, kg: Math.round((kg as number) * 10) / 10 }
  client.weights = [...client.weights.filter((other) => other.date !== date), entry]
  return ok(entry)
})

route('DELETE', '/me/weight/{date}', (state, _request, [date = '']) => {
  const client = clientOf(state, state.clientId)
  client.weights = client.weights.filter((entry) => entry.date !== date)
  return ok()
})
