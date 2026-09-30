# Coaching app

Dasha's online coaching app. She builds workouts from her exercise library and publishes them to clients. Clients open them in a Telegram Mini App, log each set and send a short report.

- Design and decisions: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Clickable prototype: https://claude.ai/artifact/WaTHWZpJ27qQq8HDozxSr5 (source in the `dashas-inst` repo, `app-prototype/`)

| Folder | What |
| --- | --- |
| `backend/` | Rust: axum, sqlx (Postgres), Telegram auth, plan import parser |
| `frontend/` | React + TypeScript + Vite: `/app` is the Mini App, `/coach` is Dasha's workspace |
| `docs/` | Architecture |

## What works so far

- **Database:** the full v1 schema (`backend/migrations/`), applied on startup.
- **Sign-in:** clients with Mini App `initData`. The coach confirms in the bot: the page shows a code, she taps Confirm in Telegram, and the page collects a JWT.
- **Bot and invites:** the webhook checks Telegram's secret header. Dasha adds a client and gets a single-use invite link that expires in 7 days. The client taps Start, their Telegram account is linked, and the bot replies with an "Open" button for the Mini App and tells Dasha.
- **Client:** `GET /me`, and `PUT /me/timezone` so reminders can use local time.
- **Coach:** `GET/POST /coach/clients`, `POST /coach/clients/{id}/invite`, and `POST /coach/import/parse`, which reads an old Telegram plan and matches it against the library.
- **Frontend:** the Mini App signs in and greets the client. The coach signs in through the bot, manages clients and invite links, and uses the import preview.
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

`/app` in a normal browser only says "open in Telegram", and `/coach` sign-in is confirmed in the bot. To use either, go through Telegram as below.

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

   Then restart the backend. It registers the webhook and logs "Telegram webhook registered".
4. Make yourself the coach, once:
   - Send `/start` to the bot. The backend logs your ID ("message from an unknown Telegram user").
   - Then run this in `psql postgres://coaching:coaching@localhost:5434/coaching`:
     ```sql
     INSERT INTO coaches (telegram_id, name) VALUES (<your id>, 'Даша');
     ```
5. Open `https://<tunnel>/coach` and choose "Увійти через Telegram". Tap Start in the bot, check that the code matches, and confirm.
6. Add a client and send the invite link to any Telegram account; your own works too. Tapping Start links it, and the bot's button opens the Mini App.

Quick tunnel URLs change on every start, so repeat step 3 each time, or set up a named Cloudflare tunnel once. BotFather's `/setdomain` is no longer needed.

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

1. Exercise library and Bunny Stream uploads.
2. Builder: whole-workout save with `version`, copy, templates, publish.
3. Client workout screens and offline set logging.
4. Finish and report, then notifications (background jobs).
5. Deploy:
   - A Render Blueprint (`render.yaml`) for the backend and Postgres in Frankfurt.
   - The frontend on Cloudflare Workers.
   - Videos on Bunny Stream.
   - About $16.50/month in total; see "Hosting and costs" in the architecture doc.
