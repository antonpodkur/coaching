# Coaching app — v1 architecture

The planned system behind the clickable prototype (`app-prototype/` in the `dashas-inst` repo). The backend is Rust and the frontend is TypeScript.

## Context

- **What v1 does:** Dasha builds workouts from her exercise library and publishes them to a client. The client opens them in a Telegram Mini App, logs each set against the target and sends a short report. Chat, technique-video checks and payments stay in Telegram or are handled by hand.
- **Scale:** one coach and tens of clients, growing to low hundreds. Performance is not a design driver. Reliability, low running cost and one developer's time are.
- **Users:** Ukraine and the Ukrainian diaspora, so clients are in many timezones. Clients train in gyms with poor reception, mostly on mid-range phones inside Telegram's in-app browser.

## Overview

```
 Client's phone                       Dasha's laptop / phone
 ┌─────────────────────┐              ┌──────────────────────┐
 │ Telegram            │              │ Browser              │
 │  ├─ chat with bot   │              │  Coach web (/coach)  │
 │  └─ Mini App (/app) │              │  Telegram login      │
 └──────┬──────────▲───┘              └─────┬──────────┬─────┘
        │ initData │ bot messages           │ API      │ resumable upload
        ▼          │                        ▼          ▼
 ┌──────────────────────────────────────────────┐  ┌────────────────────┐
 │ Backend — one Rust service (axum)            │  │ Cloudflare Stream  │
 │  REST API · Telegram auth · bot webhook ·    │──│ transcode, thumbs, │
 │  import parser · background jobs             │  │ HLS playback       │
 └──────────────┬───────────────────────────────┘  └────────────────────┘
                ▼
          ┌────────────┐
          │ Postgres   │
          └────────────┘
```

| Part | Choice | Why |
| --- | --- | --- |
| Frontend | React + TypeScript + Vite, one app with two areas: `/app` (Mini App) and `/coach` | One codebase and shared UI. The Telegram SDK and the UI libraries the builder needs are JavaScript. |
| Backend | Rust: axum, tokio, sqlx, teloxide | One small binary for the API, bot, parser and jobs. The compiler catches mistakes early. |
| API contract | `utoipa` → OpenAPI → `openapi-typescript` + `openapi-fetch` | Typed paths, params and bodies in the frontend, generated from the Rust structs. |
| Database | Postgres 16, managed (Neon, Frankfurt) | Relational data, point-in-time backups, EU region. |
| Video | Cloudflare Stream | Resumable uploads from a phone, turns iPhone HEVC into video every device plays, thumbnails, HLS. |
| Hosting | Backend on Fly.io (`fra`), frontend on Cloudflare Pages | Next to the database. HTTPS everywhere, which the Mini App requires. |

Swapping to Bunny Stream for video, or to Railway for backend and database, does not change anything else in this document.

## Frontend

- **Libraries:**
  - React Router for the two areas and their screens.
  - TanStack Query for server state, persisted to IndexedDB so an opened workout survives losing signal.
  - `dnd-kit` for reordering exercises in the builder.
  - `hls.js` where the browser cannot play HLS natively (Android WebView). iOS plays it natively.
- **Telegram:** `telegram-web-app.js`, loaded in `index.html` before the app so it can read the launch parameters from the URL. It provides `initData`, `start_param`, theme colours, the back button and haptics. Outside Telegram it does nothing.
- **Types:** `frontend/src/api/schema.ts` is generated from the backend's OpenAPI spec and never edited by hand. CI fails if it is out of date.
- **Tokens:** sent as `Authorization: Bearer …`, not cookies. Telegram Web runs Mini Apps in an iframe, where cookies are unreliable. The client token lives in memory and is re-issued from fresh `initData` on every launch. The coach token lives in `localStorage`.

## Backend

One crate, one process:

```
backend/
  src/
    main.rs          config, router, background tasks
    api/             axum handlers, split into client/ and coach/
    auth/            initData and Login Widget checks, JWT issue/verify, role extractors
    bot/             teloxide handlers (/start <invite>) and outgoing messages
    db/              sqlx queries and row types
    import/          Telegram plan parser (port of the prototype's parse_plan.py)
    jobs/            reminders and notifications
    video/           Cloudflare Stream client (upload URLs, webhook)
  migrations/        sqlx migrations, run on startup
```

- **HTTP:** axum with tower-http for CORS (locked to the frontend origin), request tracing and compression.
- **Database access:** sqlx with compile-time-checked queries. `cargo sqlx prepare` commits the query metadata so CI builds without a live database.
- **Bot:** teloxide in webhook mode, mounted in the same axum router at `/telegram/webhook`. The webhook is registered with a `secret_token`, and every request must carry a matching `X-Telegram-Bot-Api-Secret-Token` header.
- **Sessions:** HS256 JWTs (`jsonwebtoken`) carrying `role` and `client_id` or `coach_id`. Client tokens last 12 hours; coach tokens last 30 days. Rotating the secret signs everyone out.
- **Background jobs:** one tokio task that wakes every 5 minutes and sends whatever is due. v1 runs one instance. If that changes, the task takes a Postgres advisory lock first.

## Authentication and onboarding

**Client invite.** A bot can only message people who have started it, so onboarding goes through the bot:

1. Dasha adds a client in the coach web. The backend creates a random, single-use invite code that expires after 7 days.
2. She sends the link `https://t.me/<bot>?start=<code>` in her existing Telegram chat with the client.
3. The client taps Start. The bot's `/start <code>` handler links their Telegram user ID to the client record and replies with a button that opens the Mini App.

**Client sign-in.** On every launch the Mini App posts Telegram's `initData` to `POST /auth/telegram-webapp`. The backend:

1. Builds the data-check-string and verifies `hash` exactly as Telegram's Mini App docs describe. The secret key is derived from the bot token with the constant `WebAppData`.
2. Rejects `initData` older than 24 hours.
3. Looks up the client by Telegram user ID. If there is none, it answers 403, and the app asks the person to get an invite from Dasha.
4. Issues a client JWT.

The unit tests include a real `initData` string captured from the dev bot.

**Coach sign-in.** The coach web uses the Telegram Login Widget. Its payload is verified with a secret key of SHA-256(bot token). The Telegram ID must belong to a row in `coaches`. The widget needs the domain registered with BotFather (`/setdomain`).

**Authorisation.** Client handlers take `client_id` only from the token, never from the request. Coach handlers require `role = coach`.

## Data model

```sql
coaches           id, telegram_id UNIQUE, name
clients           id, coach_id, name, telegram_id UNIQUE NULL, invite_code UNIQUE NULL,
                  invite_expires_at, paid_until DATE NULL, timezone TEXT NULL,
                  created_at, archived_at
exercises         id, coach_id, name, muscle_group, aliases TEXT[],
                  video_uid NULL, video_status (none|uploading|ready|failed), archived_at
workouts          id, coach_id, client_id NULL,            -- NULL client = template
                  date DATE NULL, title, status (draft|published|done),
                  source (builder|copy|template|import), copied_from_id NULL,
                  version INT, published_at, updated_at
workout_exercises id, workout_id, exercise_id, position, per_side_label NULL, note NULL
workout_sets      id, workout_exercise_id, position,
                  target_kg NUMERIC(6,2) NULL, target_reps_min INT, target_reps_max INT,
                  actual_kg NUMERIC(6,2) NULL, actual_reps INT NULL,
                  completed_at NULL, client_updated_at NULL
workout_reports   workout_id PK, effort (easy|ok|hard), comment, finished_at, duration_min
notifications     id, kind, entity_id, local_date, sent_at,
                  UNIQUE (kind, entity_id, local_date)
```

- **Targets and results share a row.** Dasha writes the `target_*` columns and the client writes the `actual_*` columns. Tapping ✓ copies the target into the actual and sets `completed_at`. "Different from the plan" is a column comparison, so no report data is stored twice.
- **Rep ranges** are stored as min and max, so `8-10` becomes 8 and 10 and a plain `12` becomes 12 and 12. `target_kg` is NULL for bodyweight, and the numeric type keeps weights like `120.5`.
- **Copies are deep.** Copying to the next workout or from a template duplicates every exercise and set row. Clients never share rows, and templates are just workouts without a client.
- **"Минулого разу"** is the latest completed `workout_sets` for the same client and exercise. It is served by indexes on `workouts (client_id, date DESC)` and `workout_exercises (exercise_id, workout_id)`.
- **"Missed"** is not stored. It is a published workout whose date has passed without being marked done.
- **Dates:** `workouts.date` is a calendar date in the client's timezone, not a timestamp. Reminders use `clients.timezone` via `chrono-tz`. The Mini App sends the phone's timezone on first launch.
- **Exercises are archived, never deleted,** because old workouts refer to them.
- **`coach_id`** is on the top-level tables even though there is one coach. It costs nothing and keeps the door open.

## API (v1)

Client (`role = client`):

| Method | Path | Purpose |
| --- | --- | --- |
| POST | `/auth/telegram-webapp` | `initData` → token |
| GET | `/me` | Profile, this week's workouts |
| PUT | `/me/timezone` | Set once from the phone |
| GET | `/workouts/{id}` | Workout with sets, video playback info and "last time" per exercise |
| PUT | `/sets/{id}/result` | `{actual_kg, actual_reps, completed, client_updated_at}`. Idempotent. |
| POST | `/workouts/{id}/finish` | `{effort, comment}`. Marks the workout done and notifies Dasha. |

Coach (`role = coach`):

| Method | Path | Purpose |
| --- | --- | --- |
| POST | `/auth/telegram-login` | Login Widget payload → token |
| GET/POST/PATCH | `/coach/clients`, `/coach/clients/{id}` | List, add, edit (name, `paid_until`, archive) |
| POST | `/coach/clients/{id}/invite` | New invite link |
| GET | `/coach/clients/{id}/workouts` | History with results and reports |
| GET/POST/PATCH | `/coach/exercises`, `/coach/exercises/{id}` | Library |
| POST | `/coach/exercises/{id}/video-upload` | One-time Cloudflare Stream upload URL |
| POST | `/coach/workouts` | New: blank, `copy_from`, or `template_id` |
| GET/PUT | `/coach/workouts/{id}` | Whole workout as one document, with `If-Match: <version>` |
| POST | `/coach/workouts/{id}/publish` | Publish and message the client |
| POST | `/coach/import/parse` | Telegram text → preview with library matches (nothing saved) |
| POST | `/webhooks/stream` | Cloudflare Stream "video ready" (signature-checked) |
| POST | `/telegram/webhook` | Bot updates (secret-header-checked) |

- **The builder saves the whole workout.** It sends one PUT with all exercises and sets, debounced, instead of one endpoint per field. `version` is optimistic locking: if Dasha has the same workout open on her laptop and phone, the older save gets a 409 instead of silently overwriting.
- **Editing after publishing is allowed.** A set that already has a result cannot be removed without confirmation in the UI.
- **Import parsing runs on the backend.** There is one implementation, matched against the real library, and tested against her real plans in `parser/`.

## Offline set logging

Gyms often have no signal, so logging must never block on the network:

1. Opening a workout caches it (TanStack Query persisted to IndexedDB).
2. Each ✓ or edited number updates the screen immediately and goes into an outbox in IndexedDB.
3. The outbox sends set results in order whenever the app is online, and retries with backoff.
4. `PUT /sets/{id}/result` sends absolute values, not increments. The server ignores a write whose `client_updated_at` is older than the stored one. Retries and duplicates are therefore harmless.
5. "Надіслати звіт" is queued the same way. The Finish screen says "will be sent when you're online" instead of failing.

## Notifications

All messages come from the bot and are written in Ukrainian. Each send first inserts a row into `notifications`. The unique key makes restarts and duplicate job runs harmless.

| Trigger | To | Message |
| --- | --- | --- |
| Workout published | Client | "Нове тренування від Даші: Спина, вт 6 жовтня", with a button opening that workout (`startapp=w_<id>`) |
| 09:00 client time on the workout date, if published and not done | Client | Reminder with the same button |
| Workout finished | Dasha | "Максим завершив «Спина»: 2 підходи інакше, є коментар", with a link to the report |
| Published workout not opened by 20:00 on its date | Dasha | One summary message per day, not one per client |
| 3 days before `paid_until` | Dasha | Who needs to renew |

## Video pipeline

1. Dasha picks a video in the library (on a phone this is the camera roll). The frontend asks `POST /coach/exercises/{id}/video-upload` for a one-time upload URL, and the exercise becomes `uploading`.
2. The frontend uploads straight to Cloudflare Stream with tus. The upload is resumable and never passes through the backend.
3. Stream transcodes the video and calls `/webhooks/stream`. The exercise becomes `ready` with its `video_uid`.
4. Clients get the HLS manifest URL and thumbnail for each exercise in `GET /workouts/{id}`.

In v1, videos are public but only reachable by an unguessable ID. If her videos start appearing elsewhere, switch on Stream's signed URLs. The backend then signs a short-lived token per playback, and nothing else changes.

## Repository and environments

A new repository, separate from this ads repo:

```
coaching/
  backend/        Rust crate, migrations/, .sqlx/
  frontend/       Vite app: src/app, src/coach, src/shared, src/api/schema.ts
  docs/           this document, ADRs later
  docker-compose.yml   local Postgres
```

- **Local development:** Postgres in docker-compose, `cargo run` for the backend and `vite` for the frontend. Mini Apps need HTTPS, so a `cloudflared` tunnel exposes the Vite dev server, which proxies `/api` to the backend. There is a separate dev bot so real clients never see test messages. Steps are in the README.
- **CI (GitHub Actions):**
  - Backend: `cargo fmt --check`, `clippy -D warnings`, and `cargo test` against a Postgres service container.
  - Frontend: typecheck, lint and build.
  - A check that `schema.ts` matches the backend's OpenAPI output.
- **Deploy:**
  - Backend: a Docker image to Fly.io on merge to `main`. Migrations run at startup.
  - Frontend: Cloudflare Pages builds `main`.
  - Separate staging and production bots, databases and Stream keys.
- **Secrets:** `BOT_TOKEN`, `WEBHOOK_SECRET`, `JWT_SECRET`, `DATABASE_URL`, `CF_ACCOUNT_ID`, `CF_STREAM_TOKEN`, `CF_STREAM_WEBHOOK_SECRET`.

## Security and privacy

- Weights and comments like "my back hurts" are health-related data. Keep the database and backend in the EU. Rely on Neon's point-in-time restore, plus a weekly `pg_dump` to separate storage.
- Never log `initData`, tokens or the bot token. Redact the `Authorization` header in request traces.
- Rate-limit the two auth endpoints and the invite handler.
- Dasha can export or delete all of a client's data on request. A client's deletion removes their workouts, sets and reports.

## Testing

- **Backend:** integration tests with `#[sqlx::test]`, which gives each test a fresh database, for auth, copy, publish, offline-style duplicate writes and notification de-duplication.
- **Import parser:** unit tests using her real Telegram plans as fixtures (starting with `backend/tests/fixtures/example-back.txt`).
- **Auth:** `initData` and Login Widget verification against captured real payloads, plus tampered copies that must fail.
- **Frontend:** typecheck plus a few Playwright smoke tests of the builder, publish, log and report flow against a seeded backend. Before each release, check manually inside the Telegram apps on iOS and Android.

## Later, without redesign

- **v2 progression:** "Next week" suggestions come from comparing `target_*` with `actual_*` over past workouts. No schema change.
- **v3 AI drafts and Q&A:** an `ai_drafts` table for suggestions Dasha approves. Past workouts, results and report comments are the material.
- **Payments:** a `subscriptions` table replaces `paid_until` once billing moves into the app.

## Open decisions

- The app's name and domain, needed for BotFather `/setdomain` and Cloudflare Pages.
- Whether clients see only the published workout or the whole week ahead. The data model supports both.
- Signed video URLs from day one, or only if videos leak.
