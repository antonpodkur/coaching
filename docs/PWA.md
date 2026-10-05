# Installed app (PWA)

Plan from 5 October 2026. Nothing here is built yet.

## Why

Today clients and Dasha reach the app through Telegram. Telegram's home-screen shortcut (`addToHomeScreen`) is only a link: it opens Telegram, then the bot, then the Mini App. That is a lot of steps, it doesn't feel like an app, and none of an installed app's features come with it.

The goal is to install the site itself on the phone, with its own icon, its own sign-in, a start without signal, and its own notifications. The Telegram version stays.

## Decisions so far

- **Two ways in**, the Telegram Mini App and the installed app, with the same site, backend and accounts.
- **Telegram stays the identity.** All clients use Telegram, so accounts stay keyed to the Telegram ID. No passwords or emails.
- **Sign-in outside Telegram** is the bot-confirmed sign-in Dasha already uses at `/coach`, opened to clients.
- **Invites** stay Telegram app links (`startapp=inv_…`).
- **Notifications:** web push for people who turn it on, and the bot messages keep coming as well. A per-person setting comes later.
- **Dasha** installs it too and gets her notifications as push as well.
- **Address:** develop on `coaching.anton-podkur.workers.dev`, and move to the product's own domain before real clients install. See "Address".
- **Icon:** a placeholder until there is branding.

## Sign-in outside Telegram

The installed app, and any browser, opens with the saved session, or with "Увійти через Telegram" when there is none:

1. The app shows a short code and "Відкрити Telegram".
2. The bot asks "Вхід у застосунок, код 4821 — підтвердити?".
3. The person confirms and switches back. The app, which kept polling, is signed in.

**What changes:**
- `bot_login::claim` and the `coach_logins` table accept only coaches today (`NotACoach`). They would accept any Telegram account that is a coach or a joined client, and the poll would return the same coach-or-client session as `POST /auth/telegram-webapp`.
- **Sessions:** the Mini App's 12 hours suit Telegram, which signs in on every launch. The installed app needs about 30 days, like the browser workspace, renewed each time it opens. Archiving a client already ends their access, because every client request checks it.
- **Bot messages:** signing in through the bot means pressing Start, which lets the bot write to them. The "Дозволити" card is not needed on this path.
- **One screen:** the `/coach` browser sign-in and the installed app's sign-in become the same one. The backend decides the role, as the Mini App sign-in does.
- **A person who was never invited** gets the same "ask for an invite" answer as in Telegram.

**Considered and not chosen:**
- **Telegram Login Widget:** labelled legacy by Telegram. It asks for the phone number in a popup, and popups are unreliable in an installed iPhone app.
- **Telegram's OpenID Connect login:** redirects through Telegram's site. Test it in an installed iPhone app before relying on it.
- **A sign-in link sent by the bot:** on iPhone every link opens Safari, never the installed app. Safari doesn't share stored data with the installed app, so the sign-in would land in the wrong place.

## How a client gets there

1. Dasha sends the invite as today. One tap joins them and links their Telegram account.
2. In the Telegram version, a "Встанови застосунок на телефон" card replaces the Telegram home-screen card and opens the site in the phone's browser, with install steps.
3. **iPhone:** Safari → "Поділитися" → "На початковий екран". There is no install prompt, so the page shows short steps with pictures. **Android:** Chrome's own install button.
4. They open the installed app and sign in through the bot. On iPhone this has to happen after installing, because the installed app can't see what Safari stored. On Android the installed app shares Chrome's storage.
5. "Увімкнути сповіщення?" → yes.

This is a one-time setup; after that it's the icon.

## Telegram-only features in the installed app

| Inside Telegram | Installed app |
| --- | --- |
| Telegram's back arrow | Our own back arrow. `BackLink` already draws one outside Telegram. |
| Our iPhone swipe back (`gestures.ts`) | Still needed: installed iPhone apps have no swipe back. Turn it on there too. |
| Android back gesture | Works through browser history. |
| Haptics | Android: `navigator.vibrate`. iPhone: none. |
| `showConfirm` and `showAlert` | The browser's own dialogs, already the fallback in `dialogs.ts`. They work but look plain; an in-app sheet later. |
| Asking before closing during an upload | No equivalent. Logged sets are already saved on the phone; an interrupted video upload is lost. |
| `requestWriteAccess` | Not needed after a bot sign-in. |
| `shareMessage` for the invite card | The phone's share sheet (`navigator.share`) or copy; the browser branch of `InviteCard` already has copy. |
| `addToHomeScreen` | Installing replaces it. |
| Header colours, `expand`, `disableVerticalSwipes` | Colours from the manifest. Padding for the notch and home bar (`env(safe-area-inset-*)`), which Telegram handles for us today. |
| `platform` | Detected from the browser. |

## Offline

- **New:** a manifest (name, icon, colours) and a service worker that keeps the app's own files, so the installed app opens with no signal at all. Today that depends on Telegram having the page cached.
- **The same as today:** the outbox for logged sets and the saved workouts in `localStorage`. In the installed app the session is just the 30-day token.
- **Telegram's script** loads from telegram.org in `index.html` and blocks the page until it arrives, so a weak gym connection would stall the installed app. Serve it ourselves, or load it only when the launch comes from Telegram. Serve the fonts ourselves too.
- **Old versions:** a cached app can run old code for days against a newer backend. The backend has to keep accepting what older versions send, and the app shows "Оновити" when a new version is ready. This matters more because the frontend already goes live a few minutes before the backend.
- **Inside Telegram on iPhone** service workers generally don't run, so the offline start mainly helps the installed app and Android.
- **Two ways in, two copies.** The Telegram version and the installed app each keep their own offline data. Sets logged offline in one are sent from that one.
- **Videos** need a connection: they stream from Bunny. Saving them for offline is a separate feature.

## Web push

- **What it is:** notifications the installed app sends itself, like any app. They show on the lock screen with the app's icon, and a tap opens the installed app on the right screen, for example `/app/nutrition`. A bot message's button can only open the Telegram version.
- **Where it works:** on iPhone, iOS 16.4 or newer, and only once the app is installed. The app may ask only after a tap ("Увімкнути сповіщення"), never on launch. On Android it works in the browser too.
- **Backend:**
  - A VAPID key pair; the private key goes into Render.
  - A table of push subscriptions per person and device, removed when the push service reports one gone.
  - A sender.
  - A short text for each `notify::Kind`. Bot texts are too long for a banner: "Нова норма харчування · Б 100 · Ж 50 · В 200 г".
- **What stays:** the `notifications` table, its once-per-day key and its retries. A row is delivered through both channels for now.
- **App badge:** the icon can show a count (`navigator.setAppBadge`), for example for a new workout.

## Address

The installed app, its stored data, its sign-in and its push subscriptions all belong to the web address. Moving to another domain later means everyone installs again, signs in again and turns notifications on again. So:

1. Choose the product name first; with other coaches in mind it should be the product's name, not Dasha's ([MANY_COACHES.md](MANY_COACHES.md)).
2. Buy the domain.
3. Move before real clients install, following "Custom domain" in [DEPLOY.md](DEPLOY.md).

## Phases

1. **Groundwork, invisible to users.** Built:
   - **Manifest** (`frontend/public/manifest.webmanifest`): the name "Тренування" and a dumbbell icon, both placeholders until there is branding. The PNGs are made from `icon.svg` with ImageMagick: `magick -density 288 icon.svg -resize 512x512 -alpha off -depth 8 -strip icon-512.png`, and the same for `icon-192.png` and the 180-pixel `apple-touch-icon.png`. It opens at `/app`.
   - **Service worker** (`frontend/sw/sw.js`): keeps the app's own files. The build (`sw/plugin.ts`) lists them and versions it. Pages are answered from the phone's copy, so the app opens at once and with no signal. A new version waits until "Оновити" (`shared/UpdateNote.tsx`), which floats over the bottom of the screen, above Dasha's tab bar, and stays hidden over the builder's bottom bar and sheets.
   - **Telegram's script and the fonts** are served with the app.
   - **The status bar:** installed on an iPhone, the page runs under a see-through status bar, and the tops of screens leave room for it.
2. **Sign-in outside Telegram.** Built:
   - **One address, `/app`:** inside Telegram the Mini App signs in as before (`MiniApp.tsx`); anywhere else `StandaloneApp.tsx` opens with the session saved on the phone, renews it in the background (`POST /auth/refresh`), and shows the bot sign-in (`SignIn.tsx`) when there is none. `/coach/…` redirects to `/app/…`.
   - **Bot sign-in for everyone:** the coach or a client who joined; anyone else is refused at once. A client the bot could not write to yet gets the pinned welcome when they press Start. On a phone the button opens Telegram directly (`tg://`), on a computer `t.me`.
   - **Signing out:** Dasha's header has "Вийти"; a client has "Вийти з акаунта" at the bottom of their profile (the questionnaire).
   - **Replacements:** swipe back on an installed iPhone app, vibration on Android for a ticked set and a sent report (`shared/haptics.ts`), and the phone's share sheet for an invite.
3. **Getting people to install.** Built:
   - **The install page, `/install`** (`InstallPage.tsx`): steps for the phone at hand. iPhone: Share → «На початковий екран» → «Додати», with the English names too, and "open in Safari" for other browsers and for Telegram's own browser. Android: Chrome's own "Встановити" when Chrome offers it (`installPrompt.ts`), and the ⋮ menu steps. A computer: open this page on the phone. An installed app that opens on this address goes to `/app`.
   - **The card in the Telegram version** (`InstallCard.tsx`) replaces Telegram's home-screen card, on the client's home screen and Dasha's client list, on phones only. "Встановити" opens `/install` in the phone's browser (`openLink`).
   - **Once installed, never offered again:** the installed app tells the backend (`POST /auth/installed`, `app_installed_at` on clients and coaches), so the card disappears on every phone. That is also the first record of who uses the installed app.
4. **Web push.** Built:
   - **Backend:** `src/push.rs` and `api/push.rs` (the key, subscribe, unsubscribe), the `push_subscriptions` table, and `notify.rs` pushing every kind next to the bot's message, once per row (`pushed_at`). The key comes from `cargo run --bin vapid-key` and goes into Render (DEPLOY.md, 5d).
   - **Frontend:** the "Сповіщення на телефоні" card (`PushCard.tsx`) outside Telegram, on the client's home and Dasha's client list. The subscription is kept up to date each time the app opens and removed on signing out (`shared/push.ts`).
   - **Service worker:** shows each push, opens its screen on a tap (inside the open app without reloading), and puts the number waiting on the app's icon, which clears when the app is open.
   - **Both channels for now.** A per-person choice between the bot and push is a ticket for later.

## To check on real phones

- A link opened from Telegram may open in Telegram's own browser, where installing is impossible. Check, and add a "відкрий у Safari / Chrome" step to the install page if needed.
- The bot sign-in round trip on iPhone: switching to Telegram and back, and the poll picking up again.
- Opening the installed app in airplane mode.
- Push on iPhone: the permission prompt, delivery, and a tap opening the right screen.
- Video playback and uploads in the installed app, and what happens to an upload when the app goes to the background.
- The notch and home bar on phones that have them, and the see-through status bar on an installed iPhone app.
- Inside Telegram on Android, the second launch with no signal (the first one installs the service worker).
- The sign-in on an installed iPhone app: "Відкрити Telegram" (iPhone asks whether to open Telegram), Start and Confirm in the bot, then back to the app, which should be signed in a moment later.
- Swipe back from the left edge on an installed iPhone app.
- "Встановити" on the card inside Telegram: does `/install` open in Safari / Chrome, or in Telegram's own browser? Follow the steps through to the icon, and check the card is gone in Telegram afterwards.
- Chrome's own install dialog on Android from the "Встановити" button on `/install`.
- Push on an installed iPhone app: "Увімкнути" asks for permission; a new workout, a reminder and a nutrition change arrive with the right text; a tap opens the right screen; the number on the icon goes away once the app is open.
- Push for Dasha: a finished workout and a technique video open the report; the evening summary opens her client list.
- Push on Android, both in the installed app and in Chrome.
