import { useState } from 'react'

import { WeightChart } from './WeightChart'
import { formatDate, formatKg, formatShortDate } from './format'
import { CloseIcon } from './icons'
import { type WeightEntry, formatChange, weightTrend } from './weight'

/** The list shows this many weigh-ins until asked for all of them. */
const LIST_LENGTH = 10

/** The latest weight and its trend, the chart, and the weigh-ins, newest first. */
export function WeightHistory({
  entries,
  onDelete,
}: {
  entries: WeightEntry[]
  onDelete?: (date: string) => void
}) {
  const [showAll, setShowAll] = useState(false)
  const trend = weightTrend(entries)
  if (!trend) return null
  const change = formatChange(trend)
  const newest = [...entries].reverse()
  const listed = showAll ? newest : newest.slice(0, LIST_LENGTH)

  return (
    <>
      <div className="weight-now">
        <span className="weight-value">
          {formatKg(trend.latest.kg)} <span>кг</span>
        </span>
        <span className="muted small">
          {formatDate(trend.latest.date)}
          {change && ` · ${change}`}
        </span>
      </div>

      <WeightChart entries={entries} />

      <section className="stack" aria-labelledby="weigh-ins-title">
        <h2 id="weigh-ins-title" className="section-title">
          Записи
        </h2>
        <ul className="weigh-ins">
          {listed.map((entry) => (
            <li key={entry.date}>
              <span>{formatShortDate(entry.date)}</span>
              <strong>{formatKg(entry.kg)} кг</strong>
              {onDelete && (
                <button
                  type="button"
                  className="icon-button"
                  aria-label={`Видалити запис за ${formatDate(entry.date)}`}
                  onClick={() => onDelete(entry.date)}
                >
                  <CloseIcon />
                </button>
              )}
            </li>
          ))}
        </ul>
        {newest.length > LIST_LENGTH && (
          <button type="button" className="link-button" onClick={() => setShowAll(!showAll)}>
            {showAll ? 'Лише останні' : `Показати всі ${newest.length}`}
          </button>
        )}
      </section>
    </>
  )
}
