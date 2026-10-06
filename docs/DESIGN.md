# Design

How the app looks, and the rules that keep it consistent. The values live at the top of `frontend/src/styles.css`. Nothing below them uses a raw color, size or corner.

The app is a platform for coaches. Dasha is the first. The app's own look stays neutral. The coach's color (`--accent`) is the one thing that belongs to the coach.

## Colors

Each color has one job.

| Token | Job | Examples |
| --- | --- | --- |
| `--accent` (pink for Dasha) | The coach's color: today, progress, done, what is selected | Done ticks, progress bar, active tab, weight line, "Виконано" |
| `--info` (lilac) | Published, new, waiting on someone | "Опубліковано", video still processing, "Легко" |
| `--warn` (amber) | Worth a look | New report, off-the-plan chips in Dasha's report, "Важко", payment due, sets not sent yet |
| `--danger` (red) | Errors and deleting | Field errors, "Видалити…", "Архівувати…", failed upload |

- **Tints come from their color.** Use `--accent-tint`, `--warn-tint`, `--danger-tint` and the matching `-line` borders. These are mixed from the base color, so changing a coach's accent changes every tint with it.
- **Text on a tint uses the same color**, for example `--warn` on `--warn-tint`.
- **A client never sees their own result colored.** If they did 80 s instead of 90–120, the number looks like any other. Off-the-plan results are shown in amber only in the coach's report.
- **Dasha's comment to a client is neutral text in a plain card**, never pink. Pink there reads as an error.

### Surfaces, darkest to lightest

`--bg` → `--surface-sunken` (insets, fields inside cards) → `--surface` (cards, rows) → `--surface-2` (chips, filled buttons, tags) → `--surface-3` (pressed).

- **Lines** are `--line` and `--line-strong`, translucent white, so they work on any surface.
- **Dashed borders** mean "add something here" (a new set, a comment). They do not mean "missing".
- **The page color is also set outside the CSS.** If `--bg` changes, update `index.html` (`theme-color`), `public/manifest.webmanifest` and `MiniApp.tsx` (Telegram's header and background).

## Type

- **Unbounded 600** is only for screen titles, the hero card title and big numbers (weight, kcal, the sign-in code). It is very wide, so longer titles use Manrope.
- **Manrope** is used for everything else, in weights 500, 600 and 700.
- **Sizes:** `--fs-xs` 12, `--fs-sm` 13, `--fs-md` 15, `--fs-body` 16, `--fs-lg` 17, `--fs-xl` 20, `--fs-2xl` 24, `--fs-3xl` 32, `--fs-4xl` 44.
- **Every field is 16px**, because an iPhone zooms into anything smaller.
- **Labels and headings are sentence case.** No uppercase or letter-spacing. A date that starts lowercase ("вт, 6 жовтня") is capitalized by `::first-letter`.

## Corners

`--r-sm` 8 · `--r-md` 12 · `--r-lg` 16 · `--r-xl` 24 · `--r-pill`, and `50%` for circles.

| Element | Corner |
| --- | --- |
| Controls: buttons, fields, chips' containers | `--r-md` |
| Cards and rows | `--r-lg` |
| Today's card and bottom sheets | `--r-xl` |
| A control inside a card | One size down |

## Lists and sheets

- **Rows that belong together share one card** (`list-group`), with lines between them, like an iPhone's settings.
- **The client's home puts the workout first.** Anything the app asks for (messages, the questionnaire, installing, notifications) goes below the content, and is drawn with it.
- **An exercise without a video shows its number** in the workout, not an empty frame.
- **Choices with a consequence go in a bottom sheet** (`shared/Sheet.tsx`): one row per choice, each with a line saying what happens.

## Buttons

- **One white (`primary`) button per screen.** Everything else is outlined, or a frameless `link-button`.
- **A disabled button is quiet:** dark fill, faint text. Prefer keeping it enabled and explaining on tap.
- **Deleting is a red `link-button danger`** at the bottom of the page, behind a confirm.

## The coach in the client's app

Clients see who their workouts come from. The coach's photo and name (`CoachCard` on the client's profile) appear in three places:

- On today's card.
- Above a workout.
- At the top of her comments, which read like a message.

Her name comes from the data, never typed into the text.

The coach adds her photo on her profile page. Until she does, clients see her initials.

## Still to do

From the design review of 6 October 2026, everything has been done. Next ideas, not decided:

- **Coach names in the text:** the hardcoded "Даша/Даші" left in the client texts, for other coaches (docs/MANY_COACHES.md).
- **The Telegram import:** make it fill a workout instead of only previewing.
