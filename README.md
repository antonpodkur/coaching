# Coaching app

Dasha's online coaching app. She builds workouts from her exercise library and publishes them to clients. Clients open them in a Telegram Mini App, log each set and send a short report. Dasha works in the same Mini App from her phone; a browser version is there for a computer.

- Design and decisions: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Clickable prototype: https://claude.ai/artifact/WaTHWZpJ27qQq8HDozxSr5 (source in the `dashas-inst` repo, `app-prototype/`)

| Folder | What |
| --- | --- |
| `backend/` | Rust: axum, sqlx (Postgres), Telegram auth, plan import parser |
| `frontend/` | React + TypeScript + Vite: `/app` is the Mini App (the client's screens, or Dasha's workspace), `/coach` is her workspace in a browser |
| `docs/` | Architecture, and the deploy guide |
| `render.yaml` | Render Blueprint: backend and Postgres |

## What works so far

- **Database:** the full v1 schema (`backend/migrations/`), applied on startup.
- **Sign-in:** the Mini App trades `initData` for a session, and the backend decides the role: Dasha gets her workspace, invited clients get theirs. In a browser, Dasha confirms in the bot: the page shows a code, she taps Confirm in Telegram, and the page collects a JWT.
- **Bot and invites:** the webhook checks Telegram's secret header. On startup the bot's menu button is pointed at the Mini App. Dasha adds a client and sends the single-use invite link (valid 7 days) through Telegram's share sheet. The client taps Start, their Telegram account is linked, and the bot replies with a button for the Mini App and tells Dasha.
- **Client:** `GET /me`, and `PUT /me/timezone` so reminders can use local time.
- **Coach:** `GET/POST /coach/clients`, `POST /coach/clients/{id}/invite`, and `POST /coach/import/parse`, which reads an old Telegram plan and matches it against the library.
- **Editing clients:** Dasha renames a client, sets the date they have paid until (with a "+1 місяць" shortcut), and archives or restores them. The list and the client page flag a payment that has ended or ends within 3 days. An archived client is signed out at once, gets no bot messages, and can join a fresh profile later. Her phone sets her own timezone, so the evening summary follows her.
- **Workout builder:** a client's page lists their workouts; a new one starts blank or as a copy of the latest, a week later. The builder follows the phone prototype (set chips, a −/+ set editor, the library as a bottom sheet) and autosaves the whole workout with version checks. Publishing has the bot message the client once.
- **Client screens:** Today (week strip, today's or the next workout), the workout, each exercise with Dasha's video and note, "Минулого разу", and sets ticked ✓ as planned or corrected; then Finish with effort and a comment, which the bot sends to Dasha. Logging works without signal: changes queue on the phone and send themselves later.
- **Dasha's reports:** a client's page opens with their newest report, differences first (effort, comment, sets done differently or skipped), and "copy to the next workout". New reports put the client at the top of the list.
- **Background jobs** (inside the backend, every 5 minutes): workout-day reminders at 09:00 in each client's timezone, Dasha's 20:00 summary of unopened workouts and payments ending, and retries for messages Telegram refused. Run `RUST_LOG=coaching_backend=debug` to see each round.
- **Exercise library and videos:** add, rename, group and archive exercises. Videos upload from the phone straight to Bunny Stream with tus; the backend signs each upload, follows the encoding (webhook, or asking Bunny when Dasha looks), swaps the new video in when it is ready and deletes the old one.
- **Frontend:** the Mini App signs in and greets the client, or opens Dasha's workspace: a phone layout with a bottom tab bar, the client list, inviting through Telegram's share sheet, the client page and workout builder, the exercise library with video upload and playback, and the import preview. The same workspace runs in a browser at `/coach`.
- **API types:** `frontend/src/api/schema.ts` is generated from the backend's OpenAPI spec.

## Prerequisites

- Rust (stable), Node 22+ with pnpm, Docker.
- `sqlx-cli` for migrations and offline query data:
  ```bash
  cargo install sqlx-cli --no-default-features --features rustls,postgres
  ```

## Running locally

```bash
docker compose up -d db                      # Postgres on localhost:5434
cp .env.example backend/.env                 # then fill BOT_TOKEN and JWT_SECRET
cp frontend/.env.example frontend/.env.local

cd backend && cargo run                      # http://localhost:8080, runs migrations
cd frontend && pnpm install && pnpm dev      # http://localhost:5173, proxies /api to the backend
```

`/app` in a normal browser only says "open in Telegram". `/coach` works in a browser, but its sign-in is confirmed in the bot. To use either, go through Telegram as below.

### Local Telegram

Telegram only opens Mini Apps and bot buttons over HTTPS, and it has to reach the webhook. So development goes through a tunnel. Vite proxies `/api`, so one tunnel covers the frontend, the API and the webhook.

1. Create a **dev** bot with @BotFather. In `backend/.env`, set `BOT_TOKEN` and `BOT_USERNAME`, and set `WEBHOOK_SECRET` to the output of `openssl rand -hex 24`.
2. Start a tunnel. It prints an `https://….trycloudflare.com` URL.
   ```bash
   cloudflared tunnel --url http://localhost:5173
   ```
3. In `backend/.env`, set:
   - `FRONTEND_ORIGIN=https://<tunnel>`
   - `TELEGRAM_WEBHOOK_URL=https://<tunnel>/api/telegram/webhook`

   Then restart the backend. It registers the webhook, points the bot's menu button at the tunnel, and logs "Telegram webhook registered" and "menu button opens the Mini App".
4. Make yourself the coach, once: send `/start` to the bot, and the backend logs your ID ("message from an unknown Telegram user"). Set `COACH_TELEGRAM_ID` to it in `backend/.env` and restart the backend.
5. In the chat with the bot, tap the menu button ("Відкрити"). The Mini App opens your workspace.
6. Invite a client: "Запросити", enter a name, "Надіслати в Telegram", and pick a chat. Any second Telegram account works. Tapping Start there links it, and the bot's button opens the client's screens.
7. Optional, the browser version: open `https://<tunnel>/coach`, choose "Увійти через Telegram", tap Start in the bot, check that the code matches, and confirm.

Your own account is the coach, so the Mini App always opens the workspace for it, even if you also accepted an invite with it.

Quick tunnel URLs change on every start, so repeat step 3 each time, or set up a named Cloudflare tunnel once. BotFather's `/setdomain` is no longer needed.

### Local video uploads (Bunny Stream)

Without Bunny settings the library works and uploads say "not set up". To try real uploads:

1. In Bunny, create a Stream library (Frankfurt storage). Use a separate library for development.
2. From Stream › your library › API, copy the library ID, the API key, the read-only API key and the CDN hostname into the `BUNNY_*` lines of `backend/.env`, then restart the backend.
3. On the same page, set the webhook URL to `https://<tunnel>/api/webhooks/stream`. Without it, encodings still finish: the app asks Bunny whenever you look at the exercise.
4. Upload from the Mini App on a phone (Вправи → an exercise → Додати відео). Try a long video, switch apps mid-upload, and turn Wi-Fi off and on: tus should carry on where it stopped.

## Everyday commands

Backend (`backend/`):

| Command | When |
| --- | --- |
| `cargo test` | Unit tests plus route tests; each route test gets a fresh database (needs the Postgres container) |
| `cargo clippy --all-targets -- -D warnings` | Before pushing; CI runs it |
| `sqlx migrate add <name>` | New migration |
| `cargo sqlx prepare -- --all-targets` | After changing any SQL query; commit `.sqlx/`. CI builds without a database. |

Frontend (`frontend/`):

| Command | When |
| --- | --- |
| `pnpm dev` / `pnpm build` | Develop / production build |
| `pnpm typecheck` / `pnpm lint` | Before pushing; CI runs both |
| `pnpm gen:api` | After changing any backend route or body type; commit `src/api/schema.ts` |

TypeScript is pinned to 5.9. TypeScript 7 has no JavaScript API yet, and typescript-eslint and openapi-typescript need it.

## Next steps

In order, following [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md):

1. Deploy, following [docs/DEPLOY.md](docs/DEPLOY.md):
   - The backend and Postgres in Frankfurt from the Render Blueprint (`render.yaml`).
   - The frontend on Cloudflare Workers (`frontend/wrangler.jsonc`).
   - Videos on Bunny Stream.
   - About $18/month in total; see "Hosting and costs" in the architecture doc.
2. Test on real phones in Telegram, using the checklist in the deploy guide. It covers a large video upload on Dasha's iPhone, the back button, the share sheet, the menu button and logging without signal.
3. Builder extras: templates, and the Telegram import inside the builder (today it is a separate tab).
