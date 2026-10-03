# Deploying

Production runs in four places:

| Part | Where | Set up by |
| --- | --- | --- |
| Backend and Postgres | Render, Frankfurt | `render.yaml` (a Render Blueprint) |
| Frontend | Cloudflare Workers, static assets | `frontend/wrangler.jsonc`, built by Workers Builds |
| Exercise videos | Bunny Stream | A library you create by hand |
| Bot | Telegram | @BotFather; the backend sets its webhook and menu button itself |

The first deploy uses the free addresses (`*.onrender.com`, `*.workers.dev`). A custom domain can come later (see the last section), but it should be in place before real clients join.

Every step below needs your own logins, and the secrets go straight into the providers' dashboards. Never put them in the repo or in a chat.

## 1. The production bot

1. In Telegram, open @BotFather and send `/newbot`. Give it a display name (for example "Daria Khyzhniak") and a username ending in `bot`.
2. Keep the token for step 4. It goes only into Render.
3. Optional: `/setuserpic`, `/setdescription` (shown before someone taps Start) and `/setabouttext`.

This is a separate bot from the dev bot, so real clients never see test messages. Whoever creates the bot owns it. If you create it, you can hand it to Dasha later: /mybots > the bot > Bot Settings > Transfer Ownership.

You also need Dasha's numeric Telegram ID for `COACH_TELEGRAM_ID`. If you don't have it yet, leave it for step 7.

## 2. The Bunny Stream library

1. In Bunny: Stream > Add Video Library. Name it, for example, `coaching`, and choose Frankfurt for storage. Keep it separate from any development library.
2. On the library's API page, note four values: the library ID, the API key, the read-only API key and the CDN hostname.

The webhook comes in step 6, once the backend has an address.

## 3. Your Cloudflare address

In the Cloudflare dashboard, Workers & Pages shows your account's `workers.dev` subdomain. The app will be served at:

```
https://coaching.<your-subdomain>.workers.dev
```

`coaching` is the `name` in `frontend/wrangler.jsonc`. This address is `FRONTEND_ORIGIN` in the next step.

## 4. Backend and database on Render

1. Render > New > Blueprint, then connect the GitHub repo `antonpodkur/coaching`. Render reads `render.yaml` and shows a web service, `coaching-api`, and a database, `coaching-db`. Both run in Frankfurt.
2. Check the plans and prices it shows. They should be 0.5 CPU / 512 MB for the service ($7), and 0.1 CPU / 256 MB with a 5 GB disk for Postgres ($6 + $1.50). A bigger database price means `diskSizeGB` was not picked up. Don't apply it then: the disk can grow later but never shrink.
3. Fill in the values it asks for:

   | Key | Value |
   | --- | --- |
   | `WEBHOOK_SECRET` | Output of `openssl rand -hex 24` |
   | `BOT_TOKEN` | From step 1 |
   | `BOT_USERNAME` | From step 1, without `@` |
   | `FRONTEND_ORIGIN` | From step 3, without a trailing `/` |
   | `COACH_TELEGRAM_ID` | Dasha's numeric ID, or empty for now |
   | `BUNNY_STREAM_LIBRARY_ID`, `BUNNY_STREAM_API_KEY`, `BUNNY_STREAM_READ_ONLY_API_KEY`, `BUNNY_CDN_HOSTNAME` | From step 2. Leave all four empty to start without video uploads. |

   Render generates `JWT_SECRET` and fills in `DATABASE_URL`. Nobody needs to set the Telegram webhook: the backend registers `https://<its Render address>/telegram/webhook` on startup.
4. Apply. The first Docker build takes several minutes.
5. In the service's Logs, look for these lines:
   - `Telegram webhook registered`
   - `menu button opens the Mini App`
   - `listening`
6. Note the service address, for example `https://coaching-api.onrender.com`. Render adds a suffix if the name is taken. `https://<address>/health` should return `{"status":"ok","database":true}`.

The database has no access from the internet (`ipAllowList: []`). Only the backend reaches it. To run `psql` yourself, add your IP under the database's Access Control for that session, then remove it again.

## 5. Frontend on Cloudflare Workers

1. Workers & Pages > Create > Import a repository, then pick `antonpodkur/coaching`.
2. Use these settings:
   - **Project name:** `coaching`. It must match `name` in `frontend/wrangler.jsonc`.
   - **Root directory:** `frontend`
   - **Build command:** `pnpm build`
   - **Deploy command:** `npx wrangler deploy` (the default)
   - **Build variables:** `VITE_API_URL` = the Render address from step 4, without a trailing `/`. It is a build variable, not a runtime one: Vite bakes it into the app.
3. Deploy. Then open `https://coaching.<subdomain>.workers.dev/app` in a browser. It should say "Відкрий у Telegram", which means it is served.
4. Optional: under the build settings' watch paths, include only `frontend/**`, so backend-only pushes don't rebuild the frontend.

If the address differs from what you entered in step 4, change `FRONTEND_ORIGIN` in Render (Environment, then save; it redeploys).

## 6. Bunny webhook

On the library's API page in Bunny, set the webhook URL to `https://<Render address>/webhooks/stream`. With it, a video shows as ready as soon as Bunny finishes encoding. Without it, the app checks with Bunny whenever Dasha opens the exercise.

## 7. Dasha's account

- **`COACH_TELEGRAM_ID` was set:** her account already exists.
- **Not set yet:**
  1. Dasha sends `/start` to the production bot.
  2. Render's Logs show `message from an unknown Telegram user telegram_id=…`.
  3. Put that number in `COACH_TELEGRAM_ID` under Environment and save. Render redeploys, and the log says `created the coach from COACH_TELEGRAM_ID`.

Then, in her chat with the bot, the menu button "Відкрити" opens her workspace.

## 8. Check on real phones

Do this on iOS and Android, inside Telegram:

- [ ] The menu button opens the workspace for Dasha. The back arrow and the tab bar work.
- [ ] Invite a test client through the share sheet. Start links them, and the bot tells Dasha.
- [ ] Build and publish a workout. The client gets the message, and its button opens the workout.
- [ ] Log sets in airplane mode, then reconnect. The sets arrive, finishing sends Dasha the report, and the report shows differences first.
- [ ] Upload a long video from Dasha's iPhone. Switch apps mid-upload. It continues, and plays on both phones once Bunny has encoded it.
- [ ] The next morning, the reminder arrives at 09:00 local time. At 20:00 Dasha gets the summary if anything is open.

## Day to day

- **Deploys:**
  - The backend deploys when a push to `main` changes `backend/` and GitHub CI passes (`autoDeployTrigger: checksPass`).
  - The frontend deploys on each push that Workers Builds picks up.
- **Migrations** run when the new backend starts, while the old one still serves requests. Keep each migration compatible with the code before it: add columns and tables first, and remove old ones in a later deploy.
- **Logs:** Render's service Logs. `RUST_LOG` is set in `render.yaml`.
- **Rollback:** Render (Events > a previous deploy > Rollback) and Cloudflare (the Worker's Deployments). Rolling back the backend does not undo a migration.
- **Backups:** Render Postgres has point-in-time restore. A weekly off-site `pg_dump` is still to do (see the architecture doc).
- **A leaked secret:**
  - Bot token: `/revoke` in @BotFather, then update `BOT_TOKEN` in Render.
  - Bunny keys: regenerate them in Bunny, then update them in Render.
  - `JWT_SECRET`: regenerating it signs everyone out, and the Mini App signs them back in.

## Custom domain (before real clients)

The phones keep offline data and the session per address. So moving to a domain after clients have joined means a fresh sign-in, and losing any sets still waiting to send. Do it before the first real client.

1. Buy the domain at Cloudflare Registrar, so its DNS is on Cloudflare.
2. **Frontend:** the Worker > Settings > Domains & Routes > add the domain (or `app.<domain>`).
3. **Backend:**
   1. Render > the service > Settings > Custom Domains > add `api.<domain>`.
   2. Create the CNAME it asks for in Cloudflare DNS, as "DNS only" (grey cloud).
   3. Wait until Render shows the certificate.
4. **Point the two at each other:**
   1. In Render, set `FRONTEND_ORIGIN` to the new frontend address.
   2. In Workers Builds, set the `VITE_API_URL` build variable to `https://api.<domain>`.
   3. Redeploy both.

The bot's menu button follows `FRONTEND_ORIGIN` on the next start. Invite links are `t.me` links, so they don't change.
