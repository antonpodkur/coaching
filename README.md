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
- **Sign-in:** clients with Mini App `initData`; the coach with the Telegram Login Widget. Both are verified server-side and exchanged for a JWT.
- **Client:** `GET /me`, and `PUT /me/timezone` so reminders can use local time.
- **Coach:** `POST /coach/import/parse` reads an old Telegram plan and matches it against the library.
- **Frontend:** the Mini App signs in and greets the client. The coach signs in and uses the import preview.
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

`/app` in a normal browser only says "open in Telegram"; `/coach` needs the Login Widget. To use either, go through Telegram as below.

### Local Telegram

Telegram opens Mini Apps and the Login Widget only over HTTPS on a known domain, so development goes through a tunnel. Vite proxies `/api`, so one tunnel covers both frontend and backend.

1. Create a **dev** bot with @BotFather. Put its token in `backend/.env` and its username in `frontend/.env.local` (`VITE_BOT_USERNAME`).
2. Start a tunnel. It prints an `https://….trycloudflare.com` URL; set `FRONTEND_ORIGIN` in `backend/.env` to that URL and restart the backend.
   ```bash
   cloudflared tunnel --url http://localhost:5173
   ```
3. In @BotFather:
   - `/setdomain`: the tunnel domain, for the coach's Login Widget.
   - `/setmenubutton` (or `/newapp`): `https://<tunnel>/app`, for the Mini App.
4. Add yourself. Invites via the bot are not built yet, so insert rows directly. When someone without access tries to sign in, the backend logs their Telegram ID ("sign-in by a non-client" / "by a non-coach").
   ```sql
   INSERT INTO coaches (telegram_id, name) VALUES (<your id>, 'Даша');
   INSERT INTO clients (coach_id, name, telegram_id)
   SELECT id, 'Тестовий клієнт', <your id> FROM coaches WHERE telegram_id = <your id>;
   ```
   Connect with `psql postgres://coaching:coaching@localhost:5434/coaching`.

Quick tunnel URLs change on every start, so repeat step 3 each time, or set up a named Cloudflare tunnel once.

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

1. Bot and invites: teloxide webhook, `/start <code>` links a client, "Open" button.
2. Exercise library and Cloudflare Stream uploads.
3. Builder: whole-workout save with `version`, copy, templates, publish.
4. Client workout screens and offline set logging.
5. Finish and report, then notifications (background jobs).
6. Deploy: backend image (`backend/Dockerfile`) to Fly.io, Neon Postgres, frontend on Cloudflare Pages.
