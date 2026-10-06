import type { Schemas } from '../api/client'
import { CoachNote } from './CoachNote'
import { formatDate, localDate } from './format'
import { type NutritionTarget, macros } from './nutrition'

/** A daily target: calories, the three macros, and the coach's note. */
export function NutritionCard({
  target,
  coach,
}: {
  target: NutritionTarget
  coach: Schemas['CoachCard'] | null
}) {
  return (
    <>
      <div className="weight-now">
        <span className="weight-value">
          {target.kcal} <span>ккал на день</span>
        </span>
        <span className="muted small">Діє з {formatDate(localDate(new Date(target.set_at)))}</span>
      </div>
      <ul className="macros">
        {macros(target).map((macro) => (
          <li key={macro.label}>
            <span className="muted small">{macro.label}</span>
            <strong>{macro.grams} г</strong>
            <span className="muted small">{macro.kcal} ккал</span>
          </li>
        ))}
      </ul>
      {target.note && <CoachNote coach={coach} text={target.note} />}
    </>
  )
}
