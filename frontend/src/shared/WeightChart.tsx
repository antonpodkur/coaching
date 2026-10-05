import { useState } from 'react'

import { addDays, formatKg, parseDate } from './format'
import { type WeightEntry, weeklyAverages } from './weight'

const RANGES = [
  { value: 31, label: 'Місяць' },
  { value: 92, label: '3 місяці' },
  { value: 0, label: 'Усе' },
] as const
type Range = (typeof RANGES)[number]['value']

// The drawing's own units; the SVG scales to the width it gets.
const WIDTH = 340
const HEIGHT = 180
const LEFT = 36
const RIGHT = 12
const TOP = 12
const BOTTOM = 24
/** The vertical scale shows at least this many kg, so small wobbles look small. */
const MIN_SPAN_KG = 2

/** Round steps for the kg lines: the smallest that gives at most four. */
function tickStep(span: number): number {
  return [0.5, 1, 2, 5, 10, 20].find((step) => span / step <= 4) ?? 50
}

function shortDate(date: string): string {
  return parseDate(date).toLocaleDateString('uk-UA', { day: 'numeric', month: 'short' })
}

/**
 * Weight over time: each weigh-in as a dot and the weekly average as a line,
 * which is the trend worth reading. Its shape is fixed, so switching the
 * range moves nothing else on the page.
 */
export function WeightChart({ entries }: { entries: WeightEntry[] }) {
  const [range, setRange] = useState<Range>(92)
  const latest = entries.at(-1)
  const from = latest && range > 0 ? addDays(latest.date, -range) : null
  const shown = from ? entries.filter((entry) => entry.date >= from) : entries
  // Averages use the whole history, so the line starts right at the range's edge.
  const averages = weeklyAverages(entries).filter((point) => !from || point.date >= from)

  return (
    <div className="weight-chart">
      <div className="chips" role="radiogroup" aria-label="Період">
        {RANGES.map((option) => (
          <button
            key={option.value}
            type="button"
            role="radio"
            className="chip"
            aria-checked={range === option.value}
            aria-pressed={range === option.value}
            onClick={() => setRange(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
      {shown.length < 2 ? (
        <div className="weight-chart-empty">
          <p className="muted small">Ще замало записів за цей час, щоб показати графік.</p>
        </div>
      ) : (
        <Plot entries={shown} averages={averages} />
      )}
    </div>
  )
}

function Plot({
  entries,
  averages,
}: {
  entries: WeightEntry[]
  averages: { date: string; kg: number }[]
}) {
  const first = entries[0]
  const last = entries.at(-1)
  if (!first || !last) return null

  const values = [...entries.map((entry) => entry.kg), ...averages.map((point) => point.kg)]
  const low = Math.min(...values)
  const high = Math.max(...values)
  const pad = Math.max(MIN_SPAN_KG - (high - low), 0) / 2 + (high - low) * 0.12
  const bottom = low - pad
  const top = high + pad
  const step = tickStep(top - bottom)
  const ticks: number[] = []
  for (let kg = Math.ceil(bottom / step) * step; kg <= top; kg += step) ticks.push(kg)

  const start = parseDate(first.date).getTime()
  const span = Math.max(parseDate(last.date).getTime() - start, 1)
  const x = (date: string) => LEFT + ((parseDate(date).getTime() - start) / span) * (WIDTH - LEFT - RIGHT)
  const y = (kg: number) => TOP + ((top - kg) / (top - bottom)) * (HEIGHT - TOP - BOTTOM)
  const line = averages.map((point) => `${x(point.date).toFixed(1)},${y(point.kg).toFixed(1)}`).join(' ')
  const end = averages.at(-1)

  return (
    <svg
      className="weight-plot"
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      role="img"
      aria-label={`Графік ваги з ${shortDate(first.date)} до ${shortDate(last.date)}: від ${formatKg(Math.min(...entries.map((entry) => entry.kg)))} до ${formatKg(Math.max(...entries.map((entry) => entry.kg)))} кг`}
    >
      {ticks.map((kg) => (
        <g key={kg}>
          <line className="weight-grid" x1={LEFT} x2={WIDTH - RIGHT} y1={y(kg)} y2={y(kg)} />
          <text className="weight-axis" x={LEFT - 6} y={y(kg) + 4} textAnchor="end">
            {formatKg(kg)}
          </text>
        </g>
      ))}
      <text className="weight-axis" x={LEFT} y={HEIGHT - 6}>
        {shortDate(first.date)}
      </text>
      <text className="weight-axis" x={WIDTH - RIGHT} y={HEIGHT - 6} textAnchor="end">
        {shortDate(last.date)}
      </text>
      {entries.map((entry) => (
        <circle key={entry.date} className="weight-dot" cx={x(entry.date)} cy={y(entry.kg)} r={2.6} />
      ))}
      <polyline className="weight-line" points={line} />
      {end && <circle className="weight-end" cx={x(end.date)} cy={y(end.kg)} r={4} />}
    </svg>
  )
}
