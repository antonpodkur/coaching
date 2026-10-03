/** Ukrainian plural: plural(21, 'підхід', 'підходи', 'підходів') → 'підхід'. */
export function plural(n: number, one: string, few: string, many: string): string {
  const mod10 = n % 10
  const mod100 = n % 100
  if (mod10 === 1 && mod100 !== 11) return one
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return few
  return many
}

/** `29 вересня`. */
export function formatDay(iso: string): string {
  return new Date(iso).toLocaleDateString('uk-UA', { day: 'numeric', month: 'long' })
}

/** `Вівторок, 29 вересня`. */
export function formatToday(): string {
  const today = new Date().toLocaleDateString('uk-UA', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  })
  return today.charAt(0).toUpperCase() + today.slice(1)
}

/** A calendar date like `2026-10-06`, read as local time, not UTC. */
export function parseDate(date: string): Date {
  const [year = 1970, month = 1, day = 1] = date.split('-').map(Number)
  return new Date(year, month - 1, day)
}

/** `2026-10-08` → `8 жовтня`, the same in every timezone. */
export function formatDate(date: string): string {
  return parseDate(date).toLocaleDateString('uk-UA', { day: 'numeric', month: 'long' })
}

/** `2026-01-31` plus one month → `2026-02-28`: the day is kept where the month allows. */
export function addMonths(date: string, months: number): string {
  const start = parseDate(date)
  const target = new Date(start.getFullYear(), start.getMonth() + months, 1)
  const lastDay = new Date(target.getFullYear(), target.getMonth() + 1, 0).getDate()
  target.setDate(Math.min(start.getDate(), lastDay))
  return localDate(target)
}

/** Whole days from `from` to `to`, both `2026-10-06`-style dates. */
export function daysBetween(from: string, to: string): number {
  return Math.round((parseDate(to).getTime() - parseDate(from).getTime()) / 86_400_000)
}

/** `2026-10-06` → `вт, 6 жовтня`. */
export function formatShortDate(date: string): string {
  return parseDate(date).toLocaleDateString('uk-UA', {
    weekday: 'short',
    day: 'numeric',
    month: 'long',
  })
}

/** `Максим К.` → `МК`. */
export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word.charAt(0))
    .join('')
    .toUpperCase()
}

export function formatKg(kg: number): string {
  return kg.toLocaleString('uk-UA', { maximumFractionDigits: 2 })
}

/** `79 × 8-10` for weighted sets, `× 17` for bodyweight. */
export function formatSet(set: { kg?: number | null; reps_min: number; reps_max: number }): string {
  const reps = set.reps_min === set.reps_max ? `${set.reps_min}` : `${set.reps_min}-${set.reps_max}`
  return set.kg == null ? `× ${reps}` : `${formatKg(set.kg)} × ${reps}`
}

/** How an exercise is counted: kg × reps, reps (kg only as extra weight), or seconds. */
export type Measure = 'weight' | 'bodyweight' | 'time'

/** `45 с`, `1 хв`, `10 хв`, or `1:30`. */
export function formatSecs(secs: number): string {
  if (secs < 60) return `${secs} с`
  if (secs % 60 === 0) return `${secs / 60} хв`
  return `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, '0')}`
}

/** A target's count: `12`, `8–10`, or for time `45 с`, `30–45 с`, `1–2 хв`. */
export function formatCount(min: number, max: number, measure: Measure = 'weight'): string {
  if (measure !== 'time') return min === max ? `${min}` : `${min}–${max}`
  if (min === max) return formatSecs(min)
  if (max < 60) return `${min}–${max} с`
  if (min % 60 === 0 && max % 60 === 0) return `${min / 60}–${max / 60} хв`
  return `${formatSecs(min)} – ${formatSecs(max)}`
}

/**
 * A set target in the apps: `36 × 8–10`, `17 повт.` without weight, `+10 × 8`
 * with extra weight on a bodyweight exercise, and `45 с` or `+10 кг · 45 с` for time.
 */
export function formatTarget(
  kg: number | null | undefined,
  repsMin: number,
  repsMax: number,
  measure: Measure = 'weight',
): string {
  const count = formatCount(repsMin, repsMax, measure)
  if (measure === 'time') return kg == null ? count : `+${formatKg(kg)} кг · ${count}`
  if (kg == null) return `${count} повт.`
  return `${measure === 'bodyweight' ? '+' : ''}${formatKg(kg)} × ${count}`
}

/** A logged set: `34 × 12`, `12` without weight, `+10 × 8`, or `45 с`. */
export function formatDone(
  kg: number | null | undefined,
  reps: number | null | undefined,
  measure: Measure = 'weight',
): string {
  const count = reps == null ? '—' : measure === 'time' ? formatSecs(reps) : `${reps}`
  if (measure === 'time') return kg == null ? count : `+${formatKg(kg)} кг · ${count}`
  if (kg == null) return count
  return `${measure === 'bodyweight' ? '+' : ''}${formatKg(kg)} × ${count}`
}

/** Seconds as typed: `45`, or `1:30` from a minute up. */
export function formatSecsInput(secs: number): string {
  return secs < 60 ? `${secs}` : `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, '0')}`
}

/**
 * Typed time → seconds: `45`, `45 с`, `1:30`, `10 хв`. Empty is `null`;
 * unreadable or over an hour is `undefined`.
 */
export function parseSecs(text: string): number | null | undefined {
  const value = text.trim().toLowerCase()
  if (value === '') return null
  const clock = value.match(/^(\d{1,2}):([0-5]\d)$/)
  const secs = value.match(/^(\d{1,4})\s*(с|сек\.?|секунд[аи]?)?$/)
  const mins = value.match(/^(\d{1,2})\s*(хв\.?|хвилин[аи]?)$/)
  const total = clock
    ? Number(clock[1]) * 60 + Number(clock[2])
    : secs
      ? Number(secs[1])
      : mins
        ? Number(mins[1]) * 60
        : undefined
  return total !== undefined && total <= 3600 ? total : undefined
}

/** `Date` → `2026-10-06` in the phone's own timezone. */
export function localDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${date.getFullYear()}-${month}-${day}`
}

/** `сьогодні, 19:40`, `учора, 19:40`, or `6 жовтня, 19:40`. */
export function formatWhen(iso: string): string {
  const at = new Date(iso)
  const time = at.toLocaleTimeString('uk-UA', { hour: '2-digit', minute: '2-digit' })
  const day = localDate(at)
  const now = new Date()
  const yesterday = new Date(now)
  yesterday.setDate(now.getDate() - 1)
  if (day === localDate(now)) return `сьогодні, ${time}`
  if (day === localDate(yesterday)) return `учора, ${time}`
  return `${formatDay(iso)}, ${time}`
}
