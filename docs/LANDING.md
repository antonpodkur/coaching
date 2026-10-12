# Landing page

`getcoachin.app/` (Ukrainian) and `/en` (English) are the landing page for coaches. The app stays at `/app`, `/install` and `/coach/…`.

**Who it's for:** coaches, who are the ones paying. The client side is shown as something coaches get for their clients. A client who lands there finds "Я клієнт" in the header, which opens the app.

**What it asks for:** coaches message us on Telegram (`CONTACT_URL` in `frontend/landing/strings.ts`). It's free while in beta, and pricing is agreed in that chat.

## Direction

Direction A, "з переписки в застосунок":
- **Look:** the app's dark graphite, with Unbounded for headlines and Manrope for text.
- **Hero:** a workout plan sent the way coaches send it today, as a Telegram message, next to the same plan in the app's builder.
- **Threads:** one thread per exercise runs from a bracket over its lines in the message to the card it became. They are drawn in on arrival, unless the visitor's system asks for reduced motion.
- **Chat color:** slate is used only for the chat. Pink and amber appear only inside the app's pictures, where they do the same jobs as in the app.

The mockups the direction was chosen from are in [landing-mockups/](landing-mockups/):
- `a-chat-to-app` was chosen.
- `b-the-loop` is kept for later ideas. It shows the coach's and the client's phones, with the app's real notifications between them.

## How it's built

| File | What |
| --- | --- |
| `frontend/landing/strings.ts` | All text, in Ukrainian and English. The phone pictures are localized too. |
| `frontend/landing/page.ts` | The page as an HTML string. The phones and app fragments are drawn with the app's classes and sizes, not screenshots, so they can show either language. |
| `frontend/landing/landing.css` | Styles. The `.app` part copies the app's tokens from `src/styles.css`. |
| `frontend/landing/main.ts` | The page's only script: fonts, drawing the threads, and running the demo. The page reads fine without it. |
| `frontend/landing/plugin.ts` | Vite plugin. In a build it writes `dist/home.html` and `dist/en.html` with the hashed styles, script and font preloads. In `pnpm dev` and `pnpm preview` it serves them at `/` and `/en`. |
| `frontend/public/_redirects` | Cloudflare serves `/` from `home.html`. `en.html` is served at `/en` by itself. Every other page still gets the app's `index.html`. |
| `frontend/public/og-uk.png`, `og-en.png` | Link previews for Telegram, Instagram and others (1200×630), made from the hero. Remake them when the hero changes. |

The service worker (`sw/sw.js`) answers only the app's pages (`/app`, `/install`, `/coach`). It keeps the app shell under `/app`, so `/` and `/en` always come from the network and an installed app never shows an old landing page.

## Demo

Right under the hero, the real app runs twice: the coach's phone and a client's, side by side, with the steps to try between them. On narrower screens a switch shows one phone at a time.
- **How a frame knows:** `landing/main.ts` creates two iframes at `/app`, named `coachin-demo:<coach|client>:<id>`. The app sees the name (`src/demo/mode.ts`) and runs `DemoApp` instead of signing in. The name must be set when the frame is created; naming a frame already on the page doesn't reach the app inside.
- **No server:** the API client's `fetch` goes to `src/demo/transport.ts`, which answers from `src/demo/backend.ts`. That file follows the real backend's rules: what counts as off-plan, "Минулого разу", the next workout, unseen reports. Uploads, push and Telegram answer as not set up.
- **Made-up data:** `src/demo/seed.ts` dates everything around today. Coach Олена; Андрій's "Спина" for today waits as a draft; Марта has an unread report; Ігор's payment is ending; Ліза hasn't joined.
- **Linked phones:** both frames keep the data as one JSON string on the landing page's window, under their shared id. After a change, a frame says so on a `BroadcastChannel` and the other refetches. The landing page listens on the same channel to tick off the steps. "Почати спочатку" drops the data and reloads both frames.
- **Kept apart from the real app:** before any other module runs, `src/demo/isolate.ts` swaps `localStorage` and `sessionStorage` for memory, so the demo never sees a visitor's real session or queues sets into the real outbox. Each frame has its own memory router, so going back in one phone never moves the other or the page. Demo frames skip the service worker, install prompts and push, and hide scrollbars.
- **Loading:** the frames load as the section comes near. The demo's code is in its own chunks (`DemoApp`, `transport`), which the real app never loads.
- **Language:** the app is in Ukrainian only, so on `/en` the steps quote the Ukrainian button names.

Not in the demo yet: technique videos (the sample exercises have none) and the Telegram plan import.
