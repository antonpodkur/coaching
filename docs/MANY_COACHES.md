# Many coaches

Notes from 5 October 2026 on selling the app to other coaches as a subscription. Nothing here is built yet; today the app serves one coach, Dasha.

**In short:** the data is ready for many coaches, but the product around it is built for one. Most of the work is coach sign-up, names and branding, and billing, not the database.

## Already ready

- **Ownership.** `clients`, `exercises` and `workouts` carry `coach_id`. Everything else hangs off one of them: sets, reports, form videos, the questionnaire, weight, nutrition, photos.
- **Access checks.** Coach handlers take the coach from the token and check that the client, exercise or workout is theirs. Most backend test files include a case where another coach is refused.
- **Per-coach settings that exist:** name, Telegram username (for "Написати …") and timezone. The evening summary is queued for every coach in their own timezone (`queue_summaries` in `jobs.rs`).
- **The exercise library** is per coach, and names are unique per coach.
- **Storage paths** are per client and per exercise (`clients/<id>/…`, `exercises/<id>/…`), so nothing collides.
- **The bot's name**, @coach_in_bot, is already neutral.

## Built for one coach

| Area | Today | Needed |
| --- | --- | --- |
| Coach accounts | No sign-up. `coaches::ensure` creates one coach on startup from `COACH_TELEGRAM_ID`, named `DEFAULT_NAME` ("Даша"). | Sign-up, through the same bot sign-in the installed app will use ([PWA.md](PWA.md)), with a trial. A profile screen: name, photo. |
| The coach's name in texts | Written into about 40 client-facing strings: the bot (`bot/mod.rs`), notifications (`notify.rs`), the invite card (`invite_link` in `api/coach/clients.rs`: "з Дарією Хижняк") and the client screens (`frontend/src/app/`). | Taken from the coach's profile. Ukrainian needs cases and gender; see below. |
| Branding | The page title "Daria Khyzhniak — тренування" (`index.html`) and the workspace header (`CoachWorkspace.tsx`). | A product name, icon and domain. Maybe each coach's photo and colours inside. |
| One person, one coach | `clients.telegram_id` is unique across all coaches. Sign-in checks coaches first, so a coach can't also be another coach's client. | Fine at first. Allowing both would need a choice after sign-in. |
| Telegram import | The parser reads Dasha's own plan format (`import/mod.rs`). | Other coaches write plans differently: keep it as Dasha's tool, or make it more lenient. |
| Billing | None for coaches. `clients.paid_until` is a client's payment to their coach, a different thing. | Subscriptions and a trial, what clients see when their coach stops paying, a payment provider. |
| Costs and limits | One Bunny account and one server for everyone. | Video use per coach, since video is the main cost. Maybe a client or storage limit per plan. |
| Running it | Nothing. | A page for you: coaches, subscriptions, usage. |
| Legal | The privacy notes in [ARCHITECTURE.md](ARCHITECTURE.md) assume one coach. | A privacy policy and terms, who is responsible for client data (the coach or the platform), and deleting a coach's data when they leave. |

## The coach's name in Ukrainian

Today's texts use Dasha's name in several cases, with feminine verbs:

- "Нове тренування від **Даші**" (родовий)
- "**Даша оновила** твою норму харчування" (називний, feminine verb)
- "Напиши **Даші**" (давальний)
- "Онлайн-тренування з **Дарією Хижняк**" (орудний)

Two ways out:

1. **Store the forms** the texts need (називний, родовий, давальний, орудний) and the coach's gender in their profile. The coach fills them in at sign-up, with suggestions.
2. **Reword** to avoid the cases.

**Done (6 October 2026), the second way.** No client text, bot message or invite has a hardcoded name any more. The name comes from `coaches.name` and is used only in two ways:

- **As a label:** "Даша: нове тренування «Спина»", "Даша в Telegram", a comment signed with her photo and name.
- **As the subject of a verb in the present or future**, which Ukrainian does not change by gender: "Даша побачить", "бачить лише Даша", "Даша запрошує тебе".

Everything else is worded without a name: "Звіт надіслано", "Твоя норма харчування на день оновилася". Before sign-in, when the coach is unknown, texts speak of "той, хто тебе запросив". Name forms and gender are not needed for now.

## Decisions to make

- **The product name**, then the domain and icon. All three tie to the installed app; see "Address" in [PWA.md](PWA.md).
- **One app for all coaches or an app per coach.** One app shows each client their coach's name and photo inside. An app per coach gives each coach their own address (`dasha.<product>`) and their own icon and name on clients' phones, which coaches would like, but costs more work. Start with one app; an app per coach could be a premium option later.
- **One shared bot or a bot per coach.** A bot per coach shows the coach's brand in Telegram. But it needs a bot token and a webhook per coach, and sign-in has to know which bot signed the `initData`. Start with one shared bot.
- **When a coach stops paying:** read-only for their clients, a grace period, or a lock.
- **A payment provider** that works for a Ukrainian business and charges monthly.

## Rule from now on

No new hardcoded coach names. New texts take the name from the coach's profile (`coaches.name`) or avoid it.

## Possible order

1. ~~Replace the hardcoded names~~ (done by rewording, see above).
2. Coach sign-up behind an allow-list you control, with no billing yet, to try it with one or two coaches by hand.
3. An admin page and usage per coach.
4. Billing and plans.
5. Legal pages.
