import { MEASURES, type Measure } from './library'

/** How an exercise is counted: with weight, bodyweight, or time. */
export function MeasurePicker({
  value,
  onChange,
}: {
  value: Measure
  onChange: (measure: Measure) => void
}) {
  const hint = MEASURES.find((measure) => measure.value === value)?.hint
  return (
    <>
      <div className="chips wrap" role="radiogroup" aria-label="Як рахувати">
        {MEASURES.map((measure) => (
          <button
            key={measure.value}
            type="button"
            role="radio"
            className="chip"
            aria-checked={value === measure.value}
            aria-pressed={value === measure.value}
            onClick={() => onChange(measure.value)}
          >
            {measure.label}
          </button>
        ))}
      </div>
      {hint && <p className="muted small">{hint}</p>}
    </>
  )
}
