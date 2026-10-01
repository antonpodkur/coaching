import { useState } from 'react'

import { formatKg } from '../../shared/format'
import { MinusIcon, PlusIcon } from '../../shared/icons'
import { type DraftExercise, type DraftSet, formatReps, parseKg, parseReps } from './draft'

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
  const [kgText, setKgText] = useState(set.kg === null ? '' : formatKg(set.kg))
  const [repsText, setRepsText] = useState(formatReps(set))
  const kgBad = parseKg(kgText) === undefined
  const repsBad = parseReps(repsText) === undefined

  const typeKg = (text: string) => {
    setKgText(text)
    const kg = parseKg(text)
    if (kg !== undefined) onChange({ ...set, kg })
  }
  const typeReps = (text: string) => {
    setRepsText(text)
    const reps = parseReps(text)
    if (reps) onChange({ ...set, ...reps })
  }
  const stepKg = (delta: number) => {
    const kg = Math.max(0, Math.round(((set.kg ?? 0) + delta) * 100) / 100)
    const next = kg === 0 ? null : kg
    onChange({ ...set, kg: next })
    setKgText(next === null ? '' : formatKg(next))
  }
  const stepReps = (delta: number) => {
    const min = Math.max(1, set.reps_min + delta)
    const max = Math.max(min, set.reps_max + delta)
    onChange({ ...set, reps_min: min, reps_max: max })
    setRepsText(formatReps({ reps_min: min, reps_max: max }))
  }

  return (
    <section className="set-editor" aria-label="Підхід">
      <div className="set-editor-head">
        <strong>{exercise.name}</strong>
        <span className="muted small">
          підхід {setIndex + 1} з {exercise.sets.length}
        </span>
      </div>
      <div className="steppers">
        <div className="stepper-field">
          <span>Вага, кг</span>
          <div className={kgBad ? 'stepper bad' : 'stepper'}>
            <button type="button" aria-label="Менше ваги" onClick={() => stepKg(-1)}>
              <MinusIcon />
            </button>
            <input
              inputMode="decimal"
              aria-label="Вага, кг (порожньо — власна вага)"
              placeholder="—"
              value={kgText}
              onChange={(event) => typeKg(event.target.value)}
            />
            <button type="button" aria-label="Більше ваги" onClick={() => stepKg(1)}>
              <PlusIcon />
            </button>
          </div>
        </div>
        <div className="stepper-field">
          <span>Повтори</span>
          <div className={repsBad ? 'stepper bad' : 'stepper'}>
            <button type="button" aria-label="Менше повторів" onClick={() => stepReps(-1)}>
              <MinusIcon />
            </button>
            <input
              inputMode="numeric"
              aria-label="Повтори, можна діапазон 8-10"
              value={repsText}
              onChange={(event) => typeReps(event.target.value)}
            />
            <button type="button" aria-label="Більше повторів" onClick={() => stepReps(1)}>
              <PlusIcon />
            </button>
          </div>
        </div>
      </div>
      <div className="set-editor-options">
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
