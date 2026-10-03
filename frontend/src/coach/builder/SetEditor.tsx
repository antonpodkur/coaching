import { useState } from 'react'

import { formatKg } from '../../shared/format'
import { MinusIcon, PlusIcon } from '../../shared/icons'
import {
  type DraftExercise,
  type DraftSet,
  formatCountInput,
  parseKg,
  parseReps,
  parseTime,
  timeStep,
} from './draft'

const MAX_REPS = 500
const MAX_SECS = 3600

interface Props {
  exercise: DraftExercise
  set: DraftSet
  setIndex: number
  sideLabel: string
  onChange: (set: DraftSet) => void
  onToggleSide: () => void
  onCopyToAll: () => void
  onRemove: () => void
  onDone: () => void
}

/**
 * Edits one set, docked at the bottom of the screen: big −/+ buttons for the
 * gym, and the numbers can be typed too. Mount it with a `key` per set so the
 * text fields start from that set.
 *
 * Bodyweight and timed exercises show only the count (reps or time); "+ вага"
 * adds extra weight, e.g. a belt on pull-ups. "Діапазон" splits the count into
 * від and до, e.g. 10–12 reps.
 */
export function SetEditor({
  exercise,
  set,
  setIndex,
  sideLabel,
  onChange,
  onToggleSide,
  onCopyToAll,
  onRemove,
  onDone,
}: Props) {
  const measure = exercise.measure
  const timed = measure === 'time'
  const limit = timed ? MAX_SECS : MAX_REPS
  const countText = (count: number) => formatCountInput({ reps_min: count, reps_max: count }, measure)
  /** One count as typed (`12`, `45`, `1:30`), or `undefined` if unreadable. */
  const readCount = (text: string) => {
    const count = (timed ? parseTime : parseReps)(text)
    return count && count.reps_min === count.reps_max ? count.reps_min : undefined
  }
  /** −/+ on time moves by 5 s, then 15 s, then whole minutes. */
  const step = (count: number, direction: 1 | -1) =>
    timed ? direction * timeStep(direction > 0 ? count : count - 1) : direction

  const [kgText, setKgText] = useState(set.kg === null ? '' : formatKg(set.kg))
  const [range, setRange] = useState(set.reps_min !== set.reps_max)
  const [minText, setMinText] = useState(countText(set.reps_min))
  const [maxText, setMaxText] = useState(countText(set.reps_max))
  const [extraWeight, setExtraWeight] = useState(set.kg !== null)
  const showKg = measure === 'weight' || extraWeight
  const kgBad = parseKg(kgText) === undefined
  const typedMin = readCount(minText)
  const typedMax = readCount(maxText)
  const minBad = typedMin === undefined || (range && typedMin > set.reps_max)
  const maxBad = typedMax === undefined || typedMax < set.reps_min

  const typeKg = (text: string) => {
    setKgText(text)
    const kg = parseKg(text)
    if (kg !== undefined) onChange({ ...set, kg })
  }
  const stepKg = (delta: number) => {
    const kg = Math.max(0, Math.round(((set.kg ?? 0) + delta) * 100) / 100)
    const next = kg === 0 ? null : kg
    onChange({ ...set, kg: next })
    setKgText(next === null ? '' : formatKg(next))
  }

  // Without a range, the one count is both ends.
  const typeMin = (text: string) => {
    setMinText(text)
    const min = readCount(text)
    if (min === undefined) return
    if (!range) onChange({ ...set, reps_min: min, reps_max: min })
    else if (min <= set.reps_max) onChange({ ...set, reps_min: min })
  }
  const typeMax = (text: string) => {
    setMaxText(text)
    const max = readCount(text)
    if (max !== undefined && max >= set.reps_min) onChange({ ...set, reps_max: max })
  }
  const stepMin = (direction: 1 | -1) => {
    const top = range ? set.reps_max : limit
    const min = Math.min(top, Math.max(1, set.reps_min + step(set.reps_min, direction)))
    onChange({ ...set, reps_min: min, reps_max: range ? set.reps_max : min })
    setMinText(countText(min))
  }
  const stepMax = (direction: 1 | -1) => {
    const max = Math.min(limit, Math.max(set.reps_min, set.reps_max + step(set.reps_max, direction)))
    onChange({ ...set, reps_max: max })
    setMaxText(countText(max))
  }
  const toggleRange = () => {
    // On: 10 becomes 10–12 (30 s becomes 30–45 s). Off: back to the lower end.
    const max = range
      ? set.reps_min
      : Math.min(limit, set.reps_min + (timed ? 3 * timeStep(set.reps_min) : 2))
    onChange({ ...set, reps_max: max })
    setMaxText(countText(max))
    setRange(!range)
  }
  const toggleExtraWeight = () => {
    if (extraWeight) {
      onChange({ ...set, kg: null })
      setKgText('')
    }
    setExtraWeight(!extraWeight)
  }

  const unit = timed ? 'Час' : 'Повтори'
  const fields = (showKg ? 1 : 0) + (range ? 2 : 1)

  return (
    <section className="set-editor" aria-label="Підхід">
      <div className="set-editor-head">
        <strong>{exercise.name}</strong>
        <span className="muted small">
          підхід {setIndex + 1} з {exercise.sets.length}
        </span>
      </div>
      <div className={fields === 1 ? 'steppers single' : fields === 3 ? 'steppers kg-row' : 'steppers'}>
        {showKg && (
          <Stepper
            label={measure === 'weight' ? 'Вага, кг' : 'Додаткова вага, кг'}
            inputLabel="Вага, кг (порожньо — власна вага)"
            inputMode="decimal"
            placeholder="—"
            text={kgText}
            bad={kgBad}
            lessLabel="Менше ваги"
            moreLabel="Більше ваги"
            onType={typeKg}
            onStep={stepKg}
          />
        )}
        <Stepper
          label={range ? `${unit} від` : unit}
          inputLabel={
            timed ? `${range ? 'Час від' : 'Час'}: секунди або 1:30` : range ? 'Повтори від' : 'Повтори'
          }
          // `1:30` needs the colon, which number pads lack.
          inputMode={timed ? 'text' : 'numeric'}
          text={minText}
          bad={minBad}
          lessLabel={timed ? 'Менше часу' : 'Менше повторів'}
          moreLabel={timed ? 'Більше часу' : 'Більше повторів'}
          onType={typeMin}
          onStep={stepMin}
        />
        {range && (
          <Stepper
            label="до"
            inputLabel={timed ? 'Час до: секунди або 1:30' : 'Повтори до'}
            inputMode={timed ? 'text' : 'numeric'}
            text={maxText}
            bad={maxBad}
            lessLabel={timed ? 'Менше часу, до' : 'Менше повторів, до'}
            moreLabel={timed ? 'Більше часу, до' : 'Більше повторів, до'}
            onType={typeMax}
            onStep={stepMax}
          />
        )}
      </div>
      <div className="set-editor-options">
        <button type="button" className="chip" aria-pressed={range} onClick={toggleRange}>
          Діапазон
        </button>
        {measure !== 'weight' && (
          <button
            type="button"
            className="chip"
            aria-pressed={extraWeight}
            onClick={toggleExtraWeight}
          >
            + вага
          </button>
        )}
        <button
          type="button"
          className="chip"
          aria-pressed={exercise.per_side_label !== null}
          onClick={onToggleSide}
        >
          {exercise.per_side_label ?? sideLabel}
        </button>
        {exercise.sets.length > 1 && (
          <button type="button" className="chip" onClick={onCopyToAll}>
            Скопіювати в усі
          </button>
        )}
      </div>
      <div className="set-editor-actions">
        <button type="button" className="button danger-outline" onClick={onRemove}>
          Видалити підхід
        </button>
        <button type="button" className="button primary" onClick={onDone}>
          Готово
        </button>
      </div>
    </section>
  )
}

/** A number with big −/+ buttons around it. */
function Stepper({
  label,
  inputLabel,
  inputMode,
  placeholder,
  text,
  bad,
  lessLabel,
  moreLabel,
  onType,
  onStep,
}: {
  label: string
  inputLabel: string
  inputMode: 'decimal' | 'numeric' | 'text'
  placeholder?: string
  text: string
  bad: boolean
  lessLabel: string
  moreLabel: string
  onType: (text: string) => void
  onStep: (direction: 1 | -1) => void
}) {
  return (
    <div className="stepper-field">
      <span>{label}</span>
      <div className={bad ? 'stepper bad' : 'stepper'}>
        <button type="button" aria-label={lessLabel} onClick={() => onStep(-1)}>
          <MinusIcon />
        </button>
        <input
          inputMode={inputMode}
          aria-label={inputLabel}
          placeholder={placeholder}
          value={text}
          onChange={(event) => onType(event.target.value)}
        />
        <button type="button" aria-label={moreLabel} onClick={() => onStep(1)}>
          <PlusIcon />
        </button>
      </div>
    </div>
  )
}
