import { useState } from 'react'

import {
  MAX_SECS,
  TIME_UNITS,
  type TimeUnit,
  formatInUnit,
  formatKg,
  parseInUnit,
  timeUnitFor,
} from '../../shared/format'
import { MinusIcon, PlusIcon } from '../../shared/icons'
import { type DraftExercise, type DraftSet, parseKg, parseReps } from './draft'

const MAX_REPS = 500
/** −/+ on a time moves by this much in the unit it is typed in. */
const TIME_STEP: Record<TimeUnit, number> = { sec: 5, min: 60, hour: 30 * 60 }
/** Turning a time into a range adds this much on top. */
const TIME_SPAN: Record<TimeUnit, number> = { sec: 15, min: 5 * 60, hour: 30 * 60 }

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
 * від and до, e.g. 10–12 reps. Time is typed in seconds, minutes or hours, as
 * the сек / хв / год switch says: in minutes, `60` is an hour.
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
  // A time opens in the unit that shows it whole: 45 s in seconds, 20 min in minutes.
  const [unit, setUnit] = useState<TimeUnit>(() => timeUnitFor(set.reps_min, set.reps_max))
  const countText = (count: number, inUnit = unit) => (timed ? formatInUnit(count, inUnit) : `${count}`)
  /** One count as typed, or `undefined` if unreadable. */
  const readCount = (text: string) => {
    if (timed) {
      const secs = parseInUnit(text, unit)
      return secs ? secs : undefined
    }
    const reps = parseReps(text)
    return reps && reps.reps_min === reps.reps_max ? reps.reps_min : undefined
  }
  const step = timed ? TIME_STEP[unit] : 1

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
    // Down to one step (1 rep, 5 s, 1 min, half an hour), or less if typed so.
    const bottom = Math.min(step, set.reps_min)
    const min = Math.min(top, Math.max(bottom, set.reps_min + direction * step))
    onChange({ ...set, reps_min: min, reps_max: range ? set.reps_max : min })
    setMinText(countText(min))
  }
  const stepMax = (direction: 1 | -1) => {
    const max = Math.min(limit, Math.max(set.reps_min, set.reps_max + direction * step))
    onChange({ ...set, reps_max: max })
    setMaxText(countText(max))
  }
  const toggleRange = () => {
    // On: 10 becomes 10–12 (20 min becomes 20–25 min). Off: back to the lower end.
    const max = range ? set.reps_min : Math.min(limit, set.reps_min + (timed ? TIME_SPAN[unit] : 2))
    onChange({ ...set, reps_max: max })
    setMaxText(countText(max))
    setRange(!range)
  }
  // The same time, shown in another unit.
  const changeUnit = (next: TimeUnit) => {
    setUnit(next)
    setMinText(countText(set.reps_min, next))
    setMaxText(countText(set.reps_max, next))
  }
  const toggleExtraWeight = () => {
    if (extraWeight) {
      onChange({ ...set, kg: null })
      setKgText('')
    }
    setExtraWeight(!extraWeight)
  }

  const countName = timed
    ? (TIME_UNITS.find((option) => option.value === unit)?.name ?? 'Час')
    : 'Повтори'
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
          label={range ? `${countName} від` : countName}
          inputLabel={range ? `${countName} від` : countName}
          // Decimal, so `1,5` minutes can be typed.
          inputMode={timed ? 'decimal' : 'numeric'}
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
            inputLabel={`${countName} до`}
            inputMode={timed ? 'decimal' : 'numeric'}
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
        {timed && (
          <div className="unit-switch" role="radiogroup" aria-label="Одиниці часу">
            {TIME_UNITS.map((option) => (
              <button
                key={option.value}
                type="button"
                role="radio"
                aria-checked={unit === option.value}
                onClick={() => changeUnit(option.value)}
              >
                {option.short}
              </button>
            ))}
          </div>
        )}
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
          <button type="button" className="chip action" onClick={onCopyToAll}>
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
