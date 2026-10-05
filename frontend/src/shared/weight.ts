import type { Schemas } from '../api/client'
import { addDays, formatDate, formatKg } from './format'

export type WeightEntry = Schemas['WeightEntry']

/**
 * Weight swings a kilo or two from day to day with water, so trends use the
 * average of the week up to each weigh-in.
 */
const AVERAGE_DAYS = 7
/** "За місяць" compares with the average this many days before the latest weigh-in… */
const MONTH_DAYS = 28
/** …as long as there was a weigh-in within this many days; otherwise the date is named. */
const MONTH_DAYS_AT_MOST = 35

export interface WeightPoint {
  date: string
  kg: number
}

/** For each weigh-in (oldest first), the average of the week up to it. */
export function weeklyAverages(entries: WeightEntry[]): WeightPoint[] {
  return entries.map((entry, index) => {
    const from = addDays(entry.date, 1 - AVERAGE_DAYS)
    const week = entries.slice(0, index + 1).filter((other) => other.date >= from)
    return { date: entry.date, kg: week.reduce((sum, other) => sum + other.kg, 0) / week.length }
  })
}

export interface WeightTrend {
  latest: WeightEntry
  /** How the weekly average moved, in kg; `null` until there is something to compare. */
  change: number | null
  /** Measured from the first weigh-in, when there is none from a month back. */
  sinceDate: string | null
}

export function weightTrend(entries: WeightEntry[]): WeightTrend | null {
  const latest = entries.at(-1)
  if (!latest) return null
  const averages = weeklyAverages(entries)
  const now = averages.at(-1)?.kg ?? latest.kg
  const monthBack = averages.filter((point) => point.date <= addDays(latest.date, -MONTH_DAYS)).at(-1)
  if (monthBack) {
    const aMonth = monthBack.date >= addDays(latest.date, -MONTH_DAYS_AT_MOST)
    return { latest, change: now - monthBack.kg, sinceDate: aMonth ? null : monthBack.date }
  }
  const first = averages[0]
  if (!first || first.date === latest.date) return { latest, change: null, sinceDate: null }
  return { latest, change: now - first.kg, sinceDate: first.date }
}

/** `−1,2 кг за місяць`, `+0,4 кг з 3 вересня`, or `без змін за місяць`. */
export function formatChange(trend: WeightTrend): string | null {
  if (trend.change === null) return null
  const period = trend.sinceDate ? `з ${formatDate(trend.sinceDate)}` : 'за місяць'
  const rounded = Math.round(trend.change * 10) / 10
  if (rounded === 0) return `без змін ${period}`
  return `${rounded > 0 ? '+' : '−'}${formatKg(Math.abs(rounded))} кг ${period}`
}

/** `82,4 кг · −1,2 кг за місяць` for a row, or `null` with no weigh-ins. */
export function weightSummary(entries: WeightEntry[]): string | null {
  const trend = weightTrend(entries)
  if (!trend) return null
  const change = formatChange(trend)
  return `${formatKg(trend.latest.kg)} кг${change ? ` · ${change}` : ''}`
}
