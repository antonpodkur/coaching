# Coaching app — v1 architecture

The planned system behind the clickable prototype (`app-prototype/` in the `dashas-inst` repo). The backend is Rust and the frontend is TypeScript.

## Context

- **What v1 does:** Dasha builds workouts from her exercise library and publishes them to a client. The client opens them in a Telegram Mini App, logs each set against the target and sends a short report. Chat, technique-video checks and payments stay in Telegram or are handled by hand.
- **Where Dasha works:** mostly on her phone, like most coaches. Her workspace is the same Telegram Mini App: it shows her the coach screens instead of a client's. The same screens work outside Telegram too: installed on a phone's home screen, or in a browser at a computer (see [PWA.md](PWA.md)).
- **Scale:** one coach and tens of clients, growing to low hundreds. Performance is not a design driver. Reliability, low running cost and one developer's time are.
- **Users:** Ukraine and the Ukrainian diaspora, so clients are in many timezones. Clients train in gyms with poor reception, mostly on mid-range phones inside Telegram's in-app browser.

## Overview

```
 Client's phone          Dasha's phone           Outside Telegram (optional)
 ┌───────────────────┐   ┌───────────────────┐   ┌───────────────────┐
 │ Telegram          │   │ Telegram          │   │ Installed app or  │
 │  chat with bot    │   │  chat with bot    │   │  browser (/app),  │
 │  Mini App (/app)  │   │  Mini App (/app)  │   │  sign-in that is  │
 │                   │   │  (her workspace)  │   │  confirmed in bot │
 └─────────┬─────────┘   └─────────┬─────────┘   └─────────┬─────────┘
           │ initData, API         │ initData, API         │ API
           ▼                       ▼                       ▼
 ┌───────────────────────────────────────────────────────────────────┐
 │ Backend — one Rust service (axum)                                 │
 │  REST API · Telegram auth · bot webhook · import parser · jobs    │
 └────────────────┬─────────────────────────────┬────────────────────┘
                  ▼                             ▼
            ┌────────────┐           ┌────────────────────────────┐
            │ Postgres   │           │ Bunny Stream: transcoding, │
            └────────────┘           │ thumbnails, HLS playback   │
                                     └────────────────────────────┘
```

Videos go from Dasha's phone straight to Bunny with a resumable upload; the backend only signs the upload. Clients stream them from Bunny.

| Part | Choice | Why |
| --- | --- | --- |
| Frontend | React + TypeScript + Vite. `/app` is the app for clients and for Dasha: the Telegram Mini App, or the same app installed on a phone or in a browser | One codebase and shared UI. The Telegram SDK and the UI libraries the builder needs are JavaScript. |
| Backend | Rust: axum, tokio, sqlx, and a small Bot API client over reqwest | One small binary for the API, bot, parser and jobs. The compiler catches mistakes early. |
| API contract | `utoipa` → OpenAPI → `openapi-typescript` + `openapi-fetch` | Typed paths, params and bodies in the frontend, generated from the Rust structs. |
| Database | Postgres 16, managed (Render, Frankfurt) | Relational data, 3-day point-in-time restore, EU region. |
| Video | Bunny Stream (Frankfurt storage) | Resumable uploads from a phone, turns iPhone HEVC into H.264 every device plays, thumbnails, HLS with an MP4 fallback. An EU company. |
| Hosting | Backend on Render (Frankfurt), frontend on Cloudflare Workers static assets | Always on, next to the database, nothing to maintain. HTTPS everywhere, which the Mini App requires. |

### Hosting and costs

Chosen for low cost with no servers to maintain. Prices were checked on the providers' own pages on 30 September 2026 and exclude VAT.

| Part | Service | Per month |
| --- | --- | --- |
| Backend | Render web service, 0.5 CPU / 512 MB (`0.5c-512mb`, formerly Starter), Frankfurt, built from `backend/Dockerfile` | $7 |
| Postgres | Render Postgres, 0.1 CPU / 256 MB (`0.1c-256mb`, formerly Basic 256 MB), Frankfurt, with 3-day point-in-time restore and 7 days of logical backups. Storage is billed separately at $0.30/GB; the Blueprint sets 5 GB (the tier's default is 15 GB, and disks never shrink). | $7.50 |
| Frontend | Cloudflare Workers static assets, built from GitHub. The free plan allows commercial use. | $0 |
| Video | Bunny Stream: about 200 GB delivered at $0.01/GB, plus storage. $1 monthly minimum. | ~$2.50 |
| Domain | `.com` at Cloudflare Registrar, at cost. DNS has to be on Cloudflare for Workers anyway. | ~$1 ($11/year) |
| Errors, uptime, CI, Telegram | Sentry Developer, UptimeRobot, GitHub Actions (2,000 min), Bot API | $0 |
| **Total** | | **≈ $18** |

- **Growth:** at ten times the clients, video grows to about $20 and the total to about $37. The other lines barely change.
- **Bandwidth:** Render includes 5 GB/month, which only API responses use. Videos go through Bunny and the frontend through Cloudflare.
- **Telegram webhooks** need IPv4 and port 443, 80, 88 or 8443. Render's HTTPS endpoint meets both.
- **Considered and dropped:**
  - Neon: the 5-minute job keeps its compute awake, which exceeds the free tier and costs about $19/month on the paid plan.
  - Cloudflare Stream: about $11.50/month at this usage.
  - A Hetzner server: about €10 in total, but we would maintain it ourselves.

## Frontend

- **Libraries:**
  - React Router for the two areas and their screens.
  - TanStack Query for server state. The client's workouts are also kept in `localStorage`, so an opened workout survives losing signal (see "Offline set logging").
  - `dnd-kit` for reordering exercises in the builder.
  - `hls.js` where the browser cannot play HLS natively (Android WebView). iOS plays it natively. Bunny's MP4 fallback covers any WebView where HLS misbehaves.
- **Telegram:** `telegram-web-app.js`, loaded in `index.html` before the app so it can read the launch parameters from the URL. It provides `initData`, `start_param`, theme colours, the back button, haptics and `openTelegramLink`. Outside Telegram it does nothing. A copy is served with the app (`public/`, refreshed with `pnpm update:telegram`), and so are the fonts, so a weak connection can't hold the page up.
- **One Mini App, two roles:** the sign-in response says whether Dasha or a client opened `/app`, and the app mounts her workspace or the client's screens. Nothing in the URL decides the role.
- **Coach screens are phone-first:** pages above a bottom tab bar; sub-pages go back with Telegram's own back button. Outside Telegram the same components show an in-page back link, and on a wide screen the tabs move to the top. The builder's phone design (set chips with a −/+ editor, the library as a bottom sheet) is in the prototype.
- **Invites go out as a card:** in the Mini App, "Надіслати запрошення" shares a prepared message with an "Відкрити" button (`shareMessage`). Without it, "Надіслати в Telegram" opens `https://t.me/share/url?url=<invite>` with `openTelegramLink`, so Dasha picks the chat instead of copying a link. Copying stays as a fallback.
- **Types:** `frontend/src/api/schema.ts` is generated from the backend's OpenAPI spec and never edited by hand. CI fails if it is out of date.
- **Tokens:** sent as `Authorization: Bearer …`, not cookies. Telegram Web runs Mini Apps in an iframe, where cookies are unreliable. Mini App tokens, the client's and Dasha's alike, live in memory and are re-issued from fresh `initData` on every launch. Outside Telegram the session lives in `localStorage` and is renewed with `POST /auth/refresh` each time the app opens.

## Backend

One crate, one process:

```
backend/
  src/
    main.rs          config, router, background tasks
    api/             axum handlers, split into client/ and coach/
    auth/            initData checks, bot login codes, JWT issue/verify, role extractors
    bot/             update handling: invites, coach sign-in, replies
    telegram.rs      Bot API client: sendMessage, editMessageText, answerCallbackQuery,
                     setWebhook, setChatMenuButton
    invites.rs       client invite links
    codes.rs         random link secrets, stored hashed
    db/              sqlx queries and row types
    import/          Telegram plan parser (port of the prototype's parse_plan.py)
    jobs.rs          background rounds: reminders, Dasha's summary, retries
    notify.rs        every bot message the app starts: queue, build, send
    video/           upload lifecycle; stream.rs is the Bunny Stream client (create, status,
                     delete, tus signatures, webhook signatures)
  migrations/        sqlx migrations, run on startup
```

- **HTTP:** axum with tower-http for CORS (locked to the frontend origin), request tracing and compression.
- **Database access:** sqlx with compile-time-checked queries. `cargo sqlx prepare` commits the query metadata so CI builds without a live database.
- **Bot:** a small typed Bot API client over reqwest. The bot uses five methods, so teloxide's dispatcher isn't worth its weight.
  - **Webhook:** updates arrive at `/telegram/webhook` in the same axum router. When `TELEGRAM_WEBHOOK_URL` is set, the webhook is registered on startup with a `secret_token`. Requests without the matching `X-Telegram-Bot-Api-Secret-Token` header get 401.
  - **Menu button:** on startup the bot's default menu button is set to open the Mini App at `<FRONTEND_ORIGIN>/app`, so it follows the deployment (and the dev tunnel). Clients and Dasha open the app from it in any chat with the bot.
  - **Failures:** those that may be temporary answer 500, so Telegram retries. Handling is safe to repeat: tapping a spent invite again just offers the app.
  - **Tests** swap in a client that records messages instead of sending them.
- **Sessions:** HS256 JWTs (`jsonwebtoken`) carrying `role` and `client_id` or `coach_id`. Mini App tokens (either role) last 12 hours; tokens from the bot sign-in outside Telegram last 30 days, and renewing keeps a token's lifetime. Rotating the secret signs everyone out.
- **Background jobs:** one tokio task in the same process (no extra Render service), spawned at startup. Every 5 minutes it queues reminders and Dasha's summary, then sends whatever is due, retries included. Each round takes a Postgres advisory lock first, so a second instance or a deploy overlap skips the round instead of doubling it. Rounds take an explicit `now`, so tests run them on any clock.

## Authentication and onboarding

**Client invite.** One tap from Dasha's message into the app, with no bot chat and no Start on the way:

1. Dasha adds a client in her workspace. The backend creates a random, single-use invite code that expires after 7 days.
2. The invite is an app link, `https://t.me/<bot>?startapp=inv_<code>`. It opens the bot's main Mini App straight away, so the bot needs one, set in BotFather.
   - The backend also prepares it as a message card (`savePreparedInlineMessage`): Dasha's invitation with an "Відкрити" button.
   - Her Mini App shares the card with `shareMessage`. Outside Telegram, or if the card could not be prepared, she shares the plain link through Telegram's share sheet.
3. The client taps "Відкрити". The app opens, and its sign-in carries the code as `start_param`, inside the signed `initData`. The backend accepts the invite there, links the Telegram account, and tells Dasha.
4. **Messages:** the bot may only write to people who allowed it, and pressing Start used to do that implicitly. `clients.bot_allowed_at` records it.
   - **Allowed at launch:** if Telegram's launch data says they allowed it (`allows_write_to_pm`), the bot sends its welcome right away.
   - **Otherwise:** the app shows "Дозволити", which opens Telegram's own popup (`requestWriteAccess`).
   - **Writing to the bot** also allows it.
   - Until then, nothing is sent to them: no new-workout messages, no reminders.
5. **The welcome is pinned** at the top of the chat. It says what the chat is for (new workouts and reminders), has the button into the app, and offers "Написати Даші": questions go to her personal Telegram, since nobody reads the bot. Her username comes from her own Mini App sign-in.

Older `?start=inv_<code>` links still work through the bot's `/start` handler, which also sends the pinned welcome.

**The chat is the inbox, the app is where training happens.**
- Every bot message carries one button into the right screen.
- Anything a client types gets a one-line pointer to the app and to Dasha.
- On a phone, the app offers to install itself on the home screen ("Встанови застосунок"): it opens the install page, `/install`, in the phone's browser. Hidden for anyone who already uses the installed app (`app_installed`), on every phone.
- During a workout, a swipe down does not close the app (`disableVerticalSwipes`).
- Telegram asks before closing while logged sets are still waiting to send.

**Mini App sign-in, for clients and Dasha.** On every launch the Mini App posts Telegram's `initData` to `POST /auth/telegram-webapp`. The backend:

1. Builds the data-check-string and verifies `hash` exactly as Telegram's Mini App docs describe. The secret key is derived from the bot token with the constant `WebAppData`.
2. Rejects `initData` older than 24 hours.
3. Looks up the Telegram user ID in `coaches`, then in `clients`. Dasha gets a coach session and her workspace; a client gets theirs. An account that is both (Dasha testing as her own client) gets the coach session, and an invite link she opens herself is left unused.
   - **An invite in `start_param`** is accepted first.
   - **A used or expired one** answers 403 `invite_invalid`, unless the person already joined.
   - **An account that already belongs to another client** answers 403 `linked_elsewhere`.
   - **Neither the coach nor a client:** the backend answers 403 `not_invited`, and the app asks the person to get an invite from Dasha.
4. Issues a JWT for that role, valid for 12 hours.

This is as strong as the bot-confirmed sign-in below: both rest on Telegram vouching for her user ID, and `initData` cannot be forged without the bot token. It needs no confirm step, because nobody else can start this sign-in for her on another device.

The unit tests include a real `initData` string captured from the dev bot.

**Sign-in outside Telegram, confirmed through the bot.** For the app installed on a phone, or in a browser; for Dasha and for clients alike. The sign-in screen starts a login as it opens:

1. `POST /auth/bot-login` creates a random single-use code, valid for 5 minutes. It returns the link `https://t.me/<bot>?start=login_<code>` (for a computer), the same as `tg://resolve?domain=<bot>&start=login_<code>` (a phone opens Telegram straight away), a short display code such as `4821`, and a separate poll secret that stays in the app.
2. The person opens the link and taps Start. The bot's handler takes their Telegram ID from the update (Telegram vouches for it) and checks that it is the coach or a client who joined and is not archived. It replies "Увійти в застосунок? Код 4821 [Підтвердити] [Скасувати]". A client the bot could not write to yet gets the pinned welcome first: pressing Start allows it.
   - **Anyone else** gets told to open their invite first, and the login is marked `refused`, so the app says so at once instead of waiting for the code to run out.
3. Only the Confirm button approves the code. The app polls `POST /auth/bot-login/poll` with its poll secret; someone who only saw the link cannot collect the session. Once the code is approved, the app receives the coach's or the client's session a single time, and the code is spent. Polling continues while the app is hidden, and it checks again as soon as it returns: confirming means switching to Telegram, which hides the app.

**Why the confirm step and the code.** Without them, an attacker could start a login in their own app and trick someone into opening that link, which would sign the attacker in as them. Asking them to confirm, and to match the code shown on their own screen, prevents this.

**Staying signed in.** The session lasts 30 days and is kept on the phone, so the app opens with no signal. Each time it opens (and when it comes back after 12 hours) it renews the session with `POST /auth/refresh`. That fails once the client is archived, which signs them out; a month away does too. Dasha's older `/coach` address redirects to `/app`, and her older browser token renews into the new session.

This replaces the Telegram Login Widget, which Telegram now labels legacy, and which asks for a phone number in a popup that installed iPhone apps handle badly. The last step, Telegram ID to our session, would stay the same with Telegram's OpenID Connect login if it is ever needed.

**Authorisation.** Client handlers take `client_id` only from the token, never from the request. Coach handlers require `role = coach`.

## Data model

```sql
coaches           id, telegram_id UNIQUE, name, timezone (default Europe/Kyiv)
clients           id, coach_id, name, telegram_id UNIQUE NULL, bot_allowed_at NULL, invite_code_hash UNIQUE NULL,
                  invite_expires_at, paid_until DATE NULL, timezone TEXT NULL,
                  birth_year NULL, sex (female|male) NULL, height_cm NULL,   -- the questionnaire
                  avatar_path NULL, avatar_from_telegram,                    -- their photo
                  telegram_photo_id NULL, telegram_photo_checked_at NULL,
                  created_at, archived_at
gym_media         id, client_id, kind (photo|video), object_key UNIQUE, status, length_secs NULL, created_at
body_measurements id, client_id, kind (weight), value NUMERIC(6,2), measured_on DATE, created_at,
                  UNIQUE (client_id, kind, measured_on)
nutrition_targets id, client_id, protein_g, fat_g, carbs_g, note NULL, created_at   -- latest = current
exercises         id, coach_id, name, measure (weight | bodyweight | time), muscle_group, aliases TEXT[],
                  video_uid NULL, video_length_secs NULL,
                  upload_video_uid UNIQUE NULL, upload_status (uploading|processing|failed) NULL,
                  upload_started_at NULL, in_library (false = added to one workout only),
                  description NULL, archived_at
exercise_photos   id, exercise_id, path UNIQUE, created_at          -- up to 5 per exercise
workouts          id, coach_id, client_id NULL,            -- NULL client = template
                  date DATE NULL, title, status (draft|published|done),
                  source (builder|copy|template|import), copied_from_id NULL,
                  version INT, published_at, opened_at NULL, updated_at
workout_exercises id, workout_id, exercise_id, position, per_side_label NULL, note NULL
workout_sets      id, workout_exercise_id, position,
                  target_kg NUMERIC(6,2) NULL, target_reps_min INT, target_reps_max INT,
                  actual_kg NUMERIC(6,2) NULL, actual_reps INT NULL,
                  completed_at NULL, client_updated_at NULL
workout_reports   workout_id PK, effort (easy|ok|hard), comment, finished_at, duration_min,
                  seen_at NULL                          -- Dasha opened it
notifications     id, kind, entity_id, local_date, sent_at NULL, skipped,
                  attempts, last_attempt_at NULL, UNIQUE (kind, entity_id, local_date)
coach_logins      id, code_hash UNIQUE, poll_secret_hash UNIQUE, display_code, coach_id NULL,
                  status (pending|approved|cancelled|used), expires_at
```

- **Targets and results share a row.** Dasha writes the `target_*` columns and the client writes the `actual_*` columns. Tapping ✓ copies the target into the actual and sets `completed_at`. "Different from the plan" is a column comparison, so no report data is stored twice. It is defined once, as the SQL function `set_differs(workout_sets)`: a ticked set with another weight, or reps missing or outside the target range. The report view, the clients list and the bot's message all use it.
- **Rep ranges** are stored as min and max, so `8-10` becomes 8 and 10 and a plain `12` becomes 12 and 12. The numeric type keeps weights like `120.5`.
- **How an exercise is counted** is set once in the library (`exercises.measure`):
  - `weight`: kg × reps. A set without kg is allowed, for weights described in the note, like "+10 кг з кожної сторони".
  - `bodyweight`: pull-ups and the like. The builder and the client screen show reps only, and kg is optional extra weight, shown as `+10 × 8`.
  - `time`: a plank or a bike. The reps columns hold seconds, shown as `45 с`, `1:30` or `10 хв`, so saving, copying, "Минулого разу" and `set_differs()` work unchanged.
  - Switching between weight and bodyweight is always allowed. Switching to or from time is refused once the exercise has sets (`measure_in_use`), because old reps would turn into seconds. Dasha adds a separate timed exercise instead.
  - An assisted pull-up (gravitron) is an ordinary weighted exercise where kg is the machine setting.
- **Copies are deep.** Copying to the next workout or from a template duplicates every exercise and set row. Clients never share rows, and templates are just workouts without a client.
- **"Минулого разу"** is the latest completed `workout_sets` for the same client and exercise. It is served by indexes on `workouts (client_id, date DESC)` and `workout_exercises (exercise_id, workout_id)`.
- **"Missed"** is not stored. It is a published workout whose date has passed without being marked done.
- **Dates:** `workouts.date` is a calendar date in the client's timezone, not a timestamp. Reminders use `clients.timezone` via `chrono-tz`, and Dasha's summary uses `coaches.timezone`. The Mini App sends the phone's timezone whenever it differs from the stored one, for clients and for Dasha.
- **Exercises are archived, never deleted,** because old workouts refer to them.
- **An exercise's details** are Dasha's, in the library: a description (the handle, the machine, the setup) and up to 5 photos. They are kept in the photo storage zone under `exercises/<id>/` and signed like clients' photos. Clients see them in every workout with that exercise, under her video: the text, and a row of thumbnails that open full screen. A variant that changes the numbers (a wide or narrow handle) is a separate exercise, so "Минулого разу" never mixes them.
- **Clients' technique videos** (`form_videos`) sit under a workout exercise, at most 3 per exercise in a workout.
  - **Storage:** a separate Bunny library whose CDN has token authentication on. The backend signs a directory token per video (`bcdn_token=HS256-…&token_path=/<guid>/`), which covers the playlist, segments and thumbnail, and expires on the hour 6 to 7 hours out.
  - **Encoding:** followed like exercise videos. That's the library's own webhook (`/webhooks/client-videos`), the phone's "uploaded", and a check in every jobs round. A ready video sends Dasha one bot message.
  - **Seen:** opening the report marks its videos seen.
  - **Kept** until the exercise row or the client is deleted.
- **The questionnaire** is the client's own: birth year, sex and height on `clients`, all optional, and photos and videos of their gym in `gym_media` (at most 10 photos and 3 videos).
  - **Photos** are shrunk on the phone to 1600 px, which also drops the camera's metadata, and sent through the backend, which checks they are JPEGs. They sit in a private Bunny Storage zone under `clients/<id>/gym/`, and one signed directory token covers a client's photos.
  - **Videos** take the technique videos' path: straight from the phone to the private client library, encoded there, and followed by the same webhook and jobs.
  - **Dasha** sees a summary on the client page and the whole questionnaire one tap further. The client's home screen offers it until they answer anything ("Не зараз" hides the offer on that phone).
- **The avatar** is the client's: a photo they add on their profile page, cut to a 512 px square on the phone, or else a copy of their Telegram profile photo. Dasha sees it in her client list and on the client's page; without one, initials.
  - **Their own photo** is stored like a gym photo, under `clients/<id>/avatar/`, and replaces the old file. Removing it brings the Telegram photo back.
  - **The Telegram photo** is copied by the background jobs, a few clients per round, never linked: Telegram's file links carry the bot token. It is looked at again weekly; an unchanged photo is not fetched again, and one removed or hidden from bots goes away here too. It never replaces the client's own photo. A failure is tried again a day later.
- **Weight** is logged by the client, one entry a day (weighing again replaces it), in `body_measurements`, whose `kind` leaves room for other measurements. Both apps show the same chart:
  - each weigh-in as a dot and the average of the week up to it as a line, since daily weight swings with water;
  - "за місяць": the change of that weekly average against about four weeks earlier, or since a named date when the history is shorter or sparse;
  - no reminders for now.
- **Nutrition targets** are Dasha's: grams of protein, fat and carbohydrates per day, the same every day until she changes them. Calories are worked out, never stored (4/9/4 kcal per gram). A change is a new `nutrition_targets` row, so the history stays, and saving sends the client a bot message with the new numbers.
- **Clients are archived, never deleted,** so Dasha keeps their history. An archived client is signed out on their next request (the client extractor checks `archived_at`), gets no bot messages, and cannot use an invite. If they come back and Dasha adds them as a new client, the archived profile gives up the Telegram account to the new one.
- **`coach_id`** is on the top-level tables even though there is one coach. It costs nothing and keeps the door open.
- **Invite and login codes are stored hashed.** Until it is used, a code in a link works like a password.

## API (v1)

Client (`role = client`):

| Method | Path | Purpose |
| --- | --- | --- |
| POST | `/auth/telegram-webapp` | `initData` → client or coach session, decided by the Telegram user |
| GET | `/me` | Profile |
| GET | `/me/workouts` | Published and finished workouts, by date, with set progress |
| PUT | `/me/timezone` | Set once from the phone |
| GET | `/workouts/{id}` | Workout with sets, video playback info and "last time" per exercise |
| PUT | `/sets/{id}/result` | `{actual_kg, actual_reps, completed, client_updated_at}`. Idempotent. |
| POST | `/workouts/{id}/finish` | `{effort, comment, duration_min}`. Marks the workout done and notifies Dasha once; repeating it updates the report quietly. |
| POST | `/workout-exercises/{id}/videos` | Starts a technique video for Dasha: a tus ticket into the private library. Up to 3 per exercise. |
| POST | `/form-videos/{id}/uploaded` | The file is in; Bunny encodes it, then Dasha is told |
| DELETE | `/form-videos/{id}` | The client deletes their video, on Bunny too |
| GET, PUT | `/me/questionnaire` | The questionnaire; PUT saves `{birth_year, sex, height_cm}` |
| POST | `/me/gym/photos` | A gym photo as the JPEG body. Up to 10. |
| PUT, DELETE | `/me/avatar` | The client's own photo as the JPEG body, or removing it |
| POST | `/me/gym/videos` | Starts a gym video: a tus ticket into the private library. Up to 3. |
| POST | `/me/gym/videos/{id}/uploaded` | The gym video's file is in |
| DELETE | `/me/gym/{id}` | Deletes a gym photo or video, on Bunny too |
| GET | `/me/weight` | The client's weigh-ins, oldest first |
| PUT, DELETE | `/me/weight/{date}` | `{kg}` for that day, replacing any entry; or deletes it |
| GET | `/me/nutrition` | Dasha's current daily target, or `null` |

Coach (`role = coach`):

| Method | Path | Purpose |
| --- | --- | --- |
| POST | `/auth/bot-login` | New login code, bot links (`t.me` and `tg://`) and display code |
| POST | `/auth/bot-login/poll` | `{poll_secret}` → `pending`, `cancelled`, `refused`, `expired`, or `approved` with the coach's or client's session (once) |
| POST | `/auth/refresh` | A new token for the same person and lifetime, with their profile; 401 once they are archived |
| POST | `/auth/installed` | The signed-in person uses the app installed on a home screen; the Telegram version stops offering it |
| GET | `/push/key` | The public key browsers subscribe with; 503 `push_not_configured` without one |
| PUT, DELETE | `/push/subscription` | This browser gets the signed-in person's notifications, or stops getting them |
| GET/POST/PATCH | `/coach/clients`, `/coach/clients/{id}` | List (with each client's unseen reports; `?archived=true` for the archive); add (returns the first invite link); edit (name, `paid_until`, archive or restore) |
| PUT | `/coach/me/timezone` | From Dasha's phone; her evening summary follows it |
| POST | `/coach/clients/{id}/invite` | New invite link |
| GET | `/coach/clients/{id}/questionnaire` | The client's questionnaire, with signed links to their gym photos and videos |
| GET | `/coach/clients/{id}/weight` | The client's weigh-ins, oldest first |
| POST | `/coach/exercises/{id}/photos` | A photo of the exercise as the JPEG body. Up to 5. |
| DELETE | `/coach/exercise-photos/{id}` | Deletes an exercise photo, on Bunny too |
| GET, POST | `/coach/clients/{id}/nutrition` | The client's nutrition targets, newest first; POST sets a new one and the bot tells the client |
| GET | `/coach/workouts?from&to&client_id` | Every active client's workouts in a date range (at most 62 days) plus undated drafts, with the client's name: the workouts tab |
| GET | `/coach/clients/{id}/workouts` | Her workouts for this client: undated drafts first, then newest date first, with done and differing set counts and the report's effort and seen state |
| GET | `/coach/workouts/{id}/results` | Every set's target next to what the client logged, `differs` per set, and the report |
| POST | `/coach/workouts/{id}/report/seen` | Marks the report seen, so it stops showing as new |
| GET/POST/PATCH | `/coach/exercises`, `/coach/exercises/{id}` | Library; PATCH also sets the description |
| POST | `/coach/exercises/{id}/video-upload` | Creates the Bunny video and returns a tus upload ticket (endpoint, IDs, expiry, signature) |
| POST | `/coach/exercises/{id}/video-uploaded` | The phone finished uploading; Bunny encodes next |
| POST | `/coach/workouts` | New: blank, or `copy_from` (dated a week after the original unless `date` is given); `template_id` later |
| GET/PUT/DELETE | `/coach/workouts/{id}` | Whole workout as one document; PUT needs `If-Match: <version>` |
| POST | `/coach/workouts/{id}/publish` | Publish and message the client (once); needs a date and sets in every exercise |
| POST | `/coach/import/parse` | Telegram text → preview with library matches (nothing saved) |
| POST | `/webhooks/stream` | Bunny Stream state changes (signature-checked); prompts a check with Bunny's API |
| POST | `/telegram/webhook` | Bot updates (secret-header-checked) |

- **The builder saves the whole workout.** It sends one PUT with all exercises and sets, debounced (0.7 s), instead of one endpoint per field. `version` is optimistic locking: if Dasha has the same workout open on her laptop and phone, the older save gets a 409 instead of silently overwriting, and the builder offers to reload.
- **Rows keep their IDs.** The app gives new exercise and set rows their IDs (`crypto.randomUUID()`), so a retried save cannot duplicate them. The server upserts by ID, deletes rows that are gone, and refuses IDs that belong to another workout. Updating a set touches only its `target_*` columns, so results the client logged stay attached through edits.
- **Reports, differences first.** The client page opens with the newest report: effort, comment, and the exercises where a set differed from the plan or was skipped, with the rest behind "show all". Differing sets read like `36 × 10 · план 12`. "Copy to the next workout" sits under it, since that is usually her next step. Opening a new report marks it seen; until then the clients list puts that client first, marked "Новий звіт".
- **The phone builder** (as in the prototype): sets are chips; tapping one docks an editor with −/+ steppers for weight and reps, a "Діапазон" switch that splits reps into від and до, a сек / хв / год switch for timed sets (typed in that unit, so `60` in minutes is an hour; up to three hours), the per-arm/leg label and "copy to all sets". The library opens as a bottom sheet, where a missing exercise can be added by name, to the library or to this workout only (kept out of the library, its search and plan imports; it can be moved into the library later). A ⋯ menu per exercise moves it, adds a note or removes it. Telegram asks before closing while changes are unsaved.
- **Fields behave like a native app's:** every input has 16px text, so iPhone never zooms in on focus, and the page scale is fixed. Tapping a number selects it whole, so typing replaces it; tapping text puts the caret at the end (`shared/fieldFocus.ts`). On iPhones a swipe from the left edge goes back, like the back arrow (`app/gestures.ts`); Android's system back gesture already presses that arrow.
- **Editing after publishing is allowed.** A set that already has a result cannot be removed without confirmation in the UI.
- **Import parsing runs on the backend.** There is one implementation, matched against the real library, and tested against her real plans in `backend/tests/fixtures/`.

## Offline set logging

Gyms often have no signal, so logging must never block on the network:

1. Each workout screen keeps its last server copy in `localStorage` and shows it at once, then refreshes when there is a network.
2. Each ✓ or corrected number goes into an outbox (`localStorage`, so it survives Telegram closing the app) and shows on screen immediately: the screens overlay queued changes on the server copy.
3. The outbox sends changes oldest first: on every change, when the phone comes back online or to the front, and on a retry timer (2 s, 5 s, 15 s, 30 s, then every minute). A change the server refuses (4xx other than 401) is dropped; anything else waits. When it drains, the screens refresh from the server. A change the server took stays in the outbox, marked sent, until a refreshed copy on screen shows it; dropped sooner, a ticked set would blink back to unticked while the refresh loads.
4. `PUT /sets/{id}/result` sends absolute values, not increments. The server ignores a write whose `client_updated_at` is older than the stored one, so retries, duplicates and late arrivals are harmless. Results are accepted after the report too.
5. "Надіслати звіт" is queued the same way, after the sets. The Finish screen says it will be sent once the phone connects instead of failing.
6. The Mini App remembers its session for 11 hours (with the Telegram user it belongs to). If sign-in fails for lack of a network, it reuses it, so the app also opens inside the gym. Sign-in runs even when the phone reports being offline, so it can fall back instead of waiting.
7. A service worker (`frontend/sw/`) keeps the app's own files on the phone, so the app opens with no signal at all: installed on the home screen, and inside Telegram on Android. Inside Telegram on an iPhone service workers don't run, so there the app still needs Telegram to load the page. A new version downloads in the background and waits; the app offers "Оновити", and otherwise it takes over once the app is closed. See [PWA.md](PWA.md).

## Notifications

All messages come from the bot and are written in Ukrainian. The same notifications also reach the app installed on a phone as web pushes, for everyone who allowed them there (see "Web push" below). Each one is first a row in `notifications`, unique per kind, subject and date, so restarts and repeated steps never send twice.

- **Sent at once, retried later.** A request (publish, finish) queues its row and sends it right away. If Telegram fails, the row stays unsent and the background round retries it every 10 minutes, up to 8 attempts, for a day.
- **Built when sent.** The text comes from the data at sending time. A reminder for a workout finished or deleted meanwhile is skipped (`skipped`), not sent.
- **No double sends.** A delivery holds its row for 2 minutes, so a request and a round never send the same row at once.

| Trigger | To | Message |
| --- | --- | --- |
| Workout published | Client | "Нове тренування від Даші: «Спина», вт, 6 жовтня.", with a button opening the Mini App (the workout itself once client screens exist, `startapp=w_<id>`). Not sent until the client has joined and let the bot message them; publishing again after that sends it. |
| 09:00–21:00 client time on the workout date, if published, not done, and not published in the last 12 hours | Client | "Нагадування: сьогодні тренування «Спина».", with a button opening the workout. Clients without a timezone count as Kyiv. |
| Workout finished | Dasha | "Максим К.: звіт про «Спина», вт, 6 жовтня. 18 з 20 підходів · 2 інакше, ніж у плані · важко" and the comment, with a button opening that client in her workspace |
| Dasha sets a nutrition target | Client | "Даша склала тобі норму харчування на день:" (or "оновила твою норму…") with the grams of each and "Разом близько 1650 ккал.", her note, and a button opening the targets. Skipped if a newer target replaced it before sending. |
| 20:00–22:00 in Dasha's timezone, daily | Dasha | One summary: today's published workouts nobody opened (opening a workout sets `opened_at`), and clients whose `paid_until` is within 3 days. No message on a quiet day. |

### Web push

- **Who:** anyone using the installed app (or a browser outside Telegram) who tapped "Увімкнути" on the "Сповіщення на телефоні" card. An iPhone only allows it once the app is installed. Each browser is a row in `push_subscriptions`, belonging to a client or to Dasha. Signing in as someone else on that phone moves it to them; signing out removes it.
- **What:** a short title and line for each kind, without the coach's name. A tap opens the screen it is about: the workout, the report, nutrition, or Dasha's client list. The app's icon shows how many are waiting.
- **How:** `src/push.rs` encrypts each message for that browser alone (RFC 8291) and signs it with the VAPID key (RFC 8292, `VAPID_PRIVATE_KEY`). Only browsers' push services are accepted as addresses, so the server never posts to one someone made up.
- **Once, best effort:** a row is pushed on its first delivery and marked `pushed_at`. Retries for the bot do not push again. A failed push is logged, not retried, and a browser the push service reports gone is forgotten.
- **Reach:** a client counts as reachable, for reminders and for "client notified", through the bot or through the installed app.

## Video pipeline

1. Dasha picks a video on the exercise's page; on a phone this is the camera roll. The frontend calls `POST /coach/exercises/{id}/video-upload`.
   - The backend creates the video in the Bunny Stream library, titled with the exercise's name.
   - It returns a tus ticket: Bunny's endpoint, the library and video IDs, an expiry 24 hours ahead, and the signature `sha256_hex(library_id + api_key + expiry + video_id)`. The API key never leaves the server.
   - The exercise's upload becomes `uploading`. The current video, if any, stays playable.
2. The phone uploads straight to Bunny with tus-js-client, in 8 MB chunks, retrying with backoff after a dropped connection. The upload never passes through the backend. Telegram asks before closing the Mini App while it runs. If the WebView is killed, Dasha picks the file again; each attempt gets its own Bunny video.
3. When tus finishes, the phone calls `POST /coach/exercises/{id}/video-uploaded` and the upload becomes `processing`.
4. Bunny encodes the video (H.264 up to 1080p, free). Three things prompt the backend to ask Bunny's API (`GET /library/{id}/videos/{guid}`) where it is: Bunny's webhook at `/webhooks/stream`, Dasha opening the exercise, and the library list (up to 5 encodings per view). The API's answer decides:
   - status 4 (Finished): the upload becomes the exercise's `video_uid`, with its length. The previous video is deleted on Bunny.
   - status 5 or 6 (Error, UploadFailed), or the video is gone: the upload becomes `failed`.
   - anything else: still `processing`.
5. Clients get the HLS playlist and thumbnail for each exercise in `GET /workouts/{id}`. iOS plays HLS natively; elsewhere the app loads hls.js on demand.

- **Why ask the API instead of trusting the webhook:** the webhook's status numbers differ from the API's (webhook 3 is Finished, API 3 is Transcoding). A replayed or forged webhook also cannot mark anything ready, and a lost webhook only delays the change until Dasha looks.
- **Webhook signature:** Bunny signs Stream webhooks (`X-BunnyStream-Signature`, `v1`, `hmac-sha256`): lowercase hex HMAC-SHA256 of the raw body, keyed with the library's read-only API key. With `BUNNY_STREAM_READ_ONLY_API_KEY` set, unsigned or wrongly signed webhooks get 401. Webhooks for another library or for videos that are not a pending upload are ignored.
- **URLs:** `https://<cdn hostname>/<video id>/playlist.m3u8` and `/thumbnail.jpg`.
- **Without Bunny settings** the library works, uploads answer 503 `video_not_configured`, and the app says so.
- **Library settings:** Stream › library › API: set the webhook URL to `https://api.<domain>/webhooks/stream`. MP4 fallback (Encoding tab) is optional now that the app plays HLS everywhere.
- **Access:** in v1, videos are public but only reachable by an unguessable ID. If her videos start appearing elsewhere, switch on Bunny's CDN token authentication. The backend then signs a short-lived token per playback, and nothing else changes.
- **Formats:** Bunny takes iPhone `.mov` with HEVC. Formats and signing were checked against Bunny's docs on 30 September 2026.
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
  docs/DEPLOY.md  step-by-step first deploy and day-to-day operations
```

- **Local development:** Postgres in docker-compose, `cargo run` for the backend and `vite` for the frontend. Mini Apps need HTTPS, so a `cloudflared` tunnel exposes the Vite dev server, which proxies `/api` to the backend. There is a separate dev bot so real clients never see test messages. Steps are in the README.
- **CI (GitHub Actions):**
  - Backend: `cargo fmt --check`, `clippy -D warnings`, and `cargo test` against a Postgres service container.
  - Frontend: typecheck, lint and build.
  - A check that `schema.ts` matches the backend's OpenAPI output.
- **Deploy:**
  - **Backend and Postgres:** a Render Blueprint (`render.yaml`) defines both.
    - Web service: Docker from `backend/Dockerfile`, Frankfurt, `0.5c-512mb`, health check `/health`.
    - Database: `0.1c-256mb`, Postgres 16, Frankfurt, closed to the internet (`ipAllowList: []`).
    - Render deploys `main` once GitHub CI passes (`autoDeployTrigger: checksPass`) and only for changes under `backend/`. Migrations run at startup. The service reaches the database over Render's private network.
  - **Frontend:** Cloudflare Workers Builds builds `frontend/` on push (`frontend/wrangler.jsonc`, assets only, no Worker code). `assets.not_found_handling = "single-page-application"` makes every path serve the app. `public/_headers` caches the hashed `/assets/*` for a year. `VITE_API_URL` is a build variable pointing at the backend: the `onrender.com` address at first, `https://api.<domain>` once there is a domain.
  - **Bot webhook and menu button:** set on startup. The webhook URL comes from `TELEGRAM_WEBHOOK_URL`, or on Render from `RENDER_EXTERNAL_URL` (`…/telegram/webhook`). The menu button comes from `FRONTEND_ORIGIN`.
  - **Coach account:** `COACH_TELEGRAM_ID` creates Dasha's coach row on startup, so production needs no manual SQL.
  - **Environments:** development uses the dev bot and local Postgres. Production has its own bot, database and Bunny library. A staging environment can be added later as a second Render service.
- **Secrets:** `BOT_TOKEN`, `WEBHOOK_SECRET`, `BUNNY_STREAM_API_KEY` and `BUNNY_STREAM_READ_ONLY_API_KEY` are entered in the Render dashboard (`sync: false` in the Blueprint), so they never live in the repo. Render generates `JWT_SECRET` and provides `DATABASE_URL`. Other settings, entered the same way: `BOT_USERNAME`, `FRONTEND_ORIGIN`, `COACH_TELEGRAM_ID`, `BUNNY_STREAM_LIBRARY_ID`, `BUNNY_CDN_HOSTNAME`. `SENTRY_DSN` comes with error tracking, later.

## Security and privacy

- Weights and comments like "my back hurts" are health-related data. Keep the database and backend in the EU. Render Postgres provides 3-day point-in-time restore and 7 days of logical backups. A weekly off-site `pg_dump` to Cloudflare R2 (free tier) covers losing the Render account itself.
- Never log `initData`, tokens or the bot token. Redact the `Authorization` header in request traces.
- Rate-limit the auth endpoints and the invite handler.
- Dasha can export or delete all of a client's data on request. A client's deletion removes their workouts, sets and reports.

## Testing

- **Backend:** integration tests with `#[sqlx::test]`, which gives each test a fresh database, for auth, copy, publish, offline-style duplicate writes and notification de-duplication.
- **Import parser:** unit tests using her real Telegram plans as fixtures (starting with `backend/tests/fixtures/example-back.txt`).
- **Auth:** `initData` verification against captured real payloads, plus tampered copies that must fail. Mini App sign-in returns the right role, and the coach wins for an account that is both. For bot login: codes expire, work once, and only the Confirm button approves them.
- **Client side:** only their own published workouts are visible; logging is idempotent and late offline writes do not win; "last time" shows only ticked sets from earlier dates; finishing sends Dasha one report message.
- **Jobs:** rounds on a fixed clock: reminders in each client's morning (Kyiv and New York), not repeated and not right after publishing; retries after Telegram refused a message; Dasha's summary only with something to say; one instance per round via the lock.
- **Workouts:** saving as one document (row IDs and logged results survive reordering and edits), version conflicts, validation and ownership, copying to next week, and publishing (one message per workout and date; none before the client joins).
- **Bot:** updates are posted to the real webhook route, with a recording Telegram client. Covered: invites (spent, expired, replaced, one account per profile), the secret header, greetings by role, sign-in with confirm and cancel, and the webhook and menu button set on startup.
- **Video:** a fake Bunny library behind the same client covers the upload ticket and its signature, encoding to ready, replacing a video (the old one is deleted on Bunny), abandoned and failed uploads, and webhook signatures.
- **Frontend:** typecheck plus a few Playwright smoke tests of the builder, publish, log and report flow against a seeded backend. Before each release, check manually inside the Telegram apps on iOS and Android.

## Later, without redesign

- **v2 progression:** "Next week" suggestions come from comparing `target_*` with `actual_*` over past workouts. No schema change.
- **v3 AI drafts and Q&A:** an `ai_drafts` table for suggestions Dasha approves. Past workouts, results and report comments are the material.
- **Payments:** a `subscriptions` table replaces `paid_until` once billing moves into the app.

## Open decisions

- The app's name and domain, needed for BotFather and the Cloudflare zone.
- Whether clients see only the published workout or the whole week ahead. The data model supports both.
- Signed video URLs from day one, or only if videos leak.

## Risks to test early

- **Video uploads from inside Telegram.** Dasha records on her iPhone and uploads from the Mini App, which runs in Telegram's WebView. Check early, on her phone, that picking a large video from the camera roll works and that a tus upload resumes after she switches apps or loses signal.
