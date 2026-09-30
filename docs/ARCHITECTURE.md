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
 │ Backend — one Rust service (axum)            │  │ Bunny Stream       │
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
| Database | Postgres 16, managed (Render, Frankfurt) | Relational data, 3-day point-in-time restore, EU region. |
| Video | Bunny Stream (Frankfurt storage) | Resumable uploads from a phone, turns iPhone HEVC into H.264 every device plays, thumbnails, HLS with an MP4 fallback. An EU company. |
| Hosting | Backend on Render (Frankfurt), frontend on Cloudflare Workers static assets | Always on, next to the database, nothing to maintain. HTTPS everywhere, which the Mini App requires. |

### Hosting and costs

Chosen for low cost with no servers to maintain. Prices were checked on the providers' own pages on 30 September 2026 and exclude VAT.

| Part | Service | Per month |
| --- | --- | --- |
| Backend | Render web service, Starter (512 MB), Frankfurt, built from `backend/Dockerfile` | $7 |
| Postgres | Render Postgres, Basic 256 MB, Frankfurt, with 3-day point-in-time restore and 7 days of logical backups | $6 (+$0.30/GB over 1 GB) |
| Frontend | Cloudflare Workers static assets, built from GitHub. The free plan allows commercial use. | $0 |
| Video | Bunny Stream: about 200 GB delivered at $0.01/GB, plus storage. $1 monthly minimum. | ~$2.50 |
| Domain | `.com` at Cloudflare Registrar, at cost. DNS has to be on Cloudflare for Workers anyway. | ~$1 ($11/year) |
| Errors, uptime, CI, Telegram | Sentry Developer, UptimeRobot, GitHub Actions (2,000 min), Bot API | $0 |
| **Total** | | **≈ $16.50** |

- **Growth:** at ten times the clients, video grows to about $20 and the total to about $35. The other lines barely change.
- **Bandwidth:** Render includes 5 GB/month, which only API responses use. Videos go through Bunny and the frontend through Cloudflare.
- **Telegram webhooks** need IPv4 and port 443, 80, 88 or 8443. Render's HTTPS endpoint meets both.
- **Considered and dropped:**
  - Neon: the 5-minute job keeps its compute awake, which exceeds the free tier and costs about $19/month on the paid plan.
  - Cloudflare Stream: about $11.50/month at this usage.
  - A Hetzner server: about €10 in total, but we would maintain it ourselves.

## Frontend

- **Libraries:**
  - React Router for the two areas and their screens.
  - TanStack Query for server state, persisted to IndexedDB so an opened workout survives losing signal.
  - `dnd-kit` for reordering exercises in the builder.
  - `hls.js` where the browser cannot play HLS natively (Android WebView). iOS plays it natively. Bunny's MP4 fallback covers any WebView where HLS misbehaves.
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
    auth/            initData checks, bot login codes, JWT issue/verify, role extractors
    bot/             teloxide handlers (/start <invite>) and outgoing messages
    db/              sqlx queries and row types
    import/          Telegram plan parser (port of the prototype's parse_plan.py)
    jobs/            reminders and notifications
    video/           Bunny Stream client (tus upload signatures, webhook)
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

**Coach sign-in, confirmed through the bot.** The coach web shows "Увійти через Telegram":

1. `POST /auth/bot-login` creates a random single-use code, valid for 5 minutes. It returns the link `https://t.me/<bot>?start=login_<code>` and a short display code such as `4821`.
2. Dasha opens the link and taps Start. The bot's handler takes her Telegram ID from the update (Telegram vouches for it) and checks that it belongs to a row in `coaches`. It replies "Увійти в кабінет? Код 4821 [Підтвердити] [Скасувати]".
3. Only the Confirm button approves the code. The page polls `GET /auth/bot-login/{code}`. Once the code is approved, the page receives the coach JWT a single time, and the code is spent.

**Why the confirm step and the code.** Without them, an attacker could start a login in their own browser and trick Dasha into opening that link, which would sign the attacker in as her. Asking her to confirm, and to match the code shown on her own screen, prevents this.

This replaces the Telegram Login Widget, which Telegram now labels legacy; the current scaffold still uses it. Telegram's OpenID Connect login (set up in BotFather with a client ID and secret) can be added later if clients ever get a browser version. The last step, Telegram ID to our session, stays the same.

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
coach_logins      id, code_hash UNIQUE, display_code, coach_id NULL,
                  status (pending|approved|used), expires_at
```

- **Targets and results share a row.** Dasha writes the `target_*` columns and the client writes the `actual_*` columns. Tapping ✓ copies the target into the actual and sets `completed_at`. "Different from the plan" is a column comparison, so no report data is stored twice.
- **Rep ranges** are stored as min and max, so `8-10` becomes 8 and 10 and a plain `12` becomes 12 and 12. `target_kg` is NULL for bodyweight, and the numeric type keeps weights like `120.5`.
- **Copies are deep.** Copying to the next workout or from a template duplicates every exercise and set row. Clients never share rows, and templates are just workouts without a client.
- **"Минулого разу"** is the latest completed `workout_sets` for the same client and exercise. It is served by indexes on `workouts (client_id, date DESC)` and `workout_exercises (exercise_id, workout_id)`.
- **"Missed"** is not stored. It is a published workout whose date has passed without being marked done.
- **Dates:** `workouts.date` is a calendar date in the client's timezone, not a timestamp. Reminders use `clients.timezone` via `chrono-tz`. The Mini App sends the phone's timezone on first launch.
- **Exercises are archived, never deleted,** because old workouts refer to them.
- **`coach_id`** is on the top-level tables even though there is one coach. It costs nothing and keeps the door open.
- **Login codes are stored hashed.** Until it is used, the code in the link works like a password.

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
| POST | `/auth/bot-login` | New login code, bot link and display code |
| GET | `/auth/bot-login/{code}` | `pending`, or the coach token once, after she confirms in the bot |
| GET/POST/PATCH | `/coach/clients`, `/coach/clients/{id}` | List, add, edit (name, `paid_until`, archive) |
| POST | `/coach/clients/{id}/invite` | New invite link |
| GET | `/coach/clients/{id}/workouts` | History with results and reports |
| GET/POST/PATCH | `/coach/exercises`, `/coach/exercises/{id}` | Library |
| POST | `/coach/exercises/{id}/video-upload` | Creates the Bunny video and returns a short-lived tus upload signature |
| POST | `/coach/workouts` | New: blank, `copy_from`, or `template_id` |
| GET/PUT | `/coach/workouts/{id}` | Whole workout as one document, with `If-Match: <version>` |
| POST | `/coach/workouts/{id}/publish` | Publish and message the client |
| POST | `/coach/import/parse` | Telegram text → preview with library matches (nothing saved) |
| POST | `/webhooks/stream` | Bunny Stream "encoding finished" (signature-checked) |
| POST | `/telegram/webhook` | Bot updates (secret-header-checked) |

- **The builder saves the whole workout.** It sends one PUT with all exercises and sets, debounced, instead of one endpoint per field. `version` is optimistic locking: if Dasha has the same workout open on her laptop and phone, the older save gets a 409 instead of silently overwriting.
- **Editing after publishing is allowed.** A set that already has a result cannot be removed without confirmation in the UI.
- **Import parsing runs on the backend.** There is one implementation, matched against the real library, and tested against her real plans in `backend/tests/fixtures/`.

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

1. Dasha picks a video in the library; on a phone this is the camera roll. The frontend calls `POST /coach/exercises/{id}/video-upload`.
   - The backend creates the video in the Bunny Stream library.
   - It returns a short-lived tus upload signature: a SHA-256 of the library ID, API key, expiry and video ID. The API key never leaves the server.
   - The exercise becomes `uploading`.
2. The frontend uploads straight to Bunny with tus. The upload is resumable and never passes through the backend.
3. Bunny encodes the video (H.264 up to 1080p, free) and calls `/webhooks/stream`. When the status is "finished", the exercise becomes `ready` with its `video_uid`.
4. Clients get the HLS playlist URL, an MP4 fallback URL and the thumbnail for each exercise in `GET /workouts/{id}`.

- **Library settings:** switch on MP4 fallback before the first upload.
- **Access:** in v1, videos are public but only reachable by an unguessable ID. If her videos start appearing elsewhere, switch on Bunny's CDN token authentication. The backend then signs a short-lived token per playback, and nothing else changes.
- **Formats:** confirm the exact signature and webhook formats against Bunny's docs when building this.
- **Tip for Dasha:** iPhone Camera → Formats → "Most Compatible" records H.264, which uploads faster and plays everywhere.

## Repository and environments

Separate from the `dashas-inst` ads repo:

```
coaching/
  backend/        Rust crate, migrations/, .sqlx/
  frontend/       Vite app: src/app, src/coach, src/shared, src/api/schema.ts
  docs/           this document, ADRs later
  docker-compose.yml   local Postgres
  render.yaml     Render Blueprint: backend service + Postgres
```

- **Local development:** Postgres in docker-compose, `cargo run` for the backend and `vite` for the frontend. Mini Apps need HTTPS, so a `cloudflared` tunnel exposes the Vite dev server, which proxies `/api` to the backend. There is a separate dev bot so real clients never see test messages. Steps are in the README.
- **CI (GitHub Actions):**
  - Backend: `cargo fmt --check`, `clippy -D warnings`, and `cargo test` against a Postgres service container.
  - Frontend: typecheck, lint and build.
  - A check that `schema.ts` matches the backend's OpenAPI output.
- **Deploy:**
  - **Backend and Postgres:** a Render Blueprint (`render.yaml`) defines both.
    - Web service: Docker from `backend/Dockerfile`, Frankfurt, Starter, health check `/health`.
    - Database: Basic 256 MB, Postgres 16, Frankfurt.
    - Render deploys `main` automatically, and migrations run at startup. The service reaches the database over Render's private network.
  - **Frontend:** Cloudflare Workers builds `frontend/` on push. `assets.not_found_handling = "single-page-application"` makes every path serve the app, and `VITE_API_URL` points at `https://api.<domain>`.
  - **Bot webhook:** set once per environment to `https://api.<domain>/telegram/webhook`.
  - **Environments:** development uses the dev bot and local Postgres. Production has its own bot, database and Bunny library. A staging environment can be added later as a second Render service.
- **Secrets:** `BOT_TOKEN`, `WEBHOOK_SECRET`, `JWT_SECRET`, `DATABASE_URL`, `BUNNY_STREAM_LIBRARY_ID`, `BUNNY_STREAM_API_KEY`, `BUNNY_CDN_HOSTNAME`, `SENTRY_DSN`. They are set in the Render dashboard and marked `sync: false` in the Blueprint, so they never live in the repo.

## Security and privacy

- Weights and comments like "my back hurts" are health-related data. Keep the database and backend in the EU. Render Postgres provides 3-day point-in-time restore and 7 days of logical backups. A weekly off-site `pg_dump` to Cloudflare R2 (free tier) covers losing the Render account itself.
- Never log `initData`, tokens or the bot token. Redact the `Authorization` header in request traces.
- Rate-limit the auth endpoints and the invite handler.
- Dasha can export or delete all of a client's data on request. A client's deletion removes their workouts, sets and reports.

## Testing

- **Backend:** integration tests with `#[sqlx::test]`, which gives each test a fresh database, for auth, copy, publish, offline-style duplicate writes and notification de-duplication.
- **Import parser:** unit tests using her real Telegram plans as fixtures (starting with `backend/tests/fixtures/example-back.txt`).
- **Auth:** `initData` verification against captured real payloads, plus tampered copies that must fail. For bot login: codes expire, work once, and only the Confirm button approves them.
- **Frontend:** typecheck plus a few Playwright smoke tests of the builder, publish, log and report flow against a seeded backend. Before each release, check manually inside the Telegram apps on iOS and Android.

## Later, without redesign

- **v2 progression:** "Next week" suggestions come from comparing `target_*` with `actual_*` over past workouts. No schema change.
- **v3 AI drafts and Q&A:** an `ai_drafts` table for suggestions Dasha approves. Past workouts, results and report comments are the material.
- **Payments:** a `subscriptions` table replaces `paid_until` once billing moves into the app.

## Open decisions

- The app's name and domain, needed for BotFather and the Cloudflare zone.
- Whether clients see only the published workout or the whole week ahead. The data model supports both.
- Signed video URLs from day one, or only if videos leak.
