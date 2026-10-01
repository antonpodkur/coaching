import { useState } from 'react'

import { CameraIcon, MoreIcon, PlusIcon } from '../../shared/icons'
import { type DraftExercise, setLabel } from './draft'

interface Props {
  exercise: DraftExercise
  index: number
  count: number
  /** The set open in the editor, if it belongs to this exercise. */
  selectedSet: number | null
  onSelectSet: (index: number) => void
  onAddSet: () => void
  onMove: (delta: -1 | 1) => void
  onNote: (note: string | null) => void
  onRemove: () => void
}

/** One exercise in the builder: its sets as chips, and a ⋯ menu for the rest. */
export function ExerciseCard({
  exercise,
  index,
  count,
  selectedSet,
  onSelectSet,
  onAddSet,
  onMove,
  onNote,
  onRemove,
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false)
  const [noteOpen, setNoteOpen] = useState(exercise.note !== null)
  const closeMenuAnd = (action: () => void) => () => {
    setMenuOpen(false)
    action()
  }

  return (
    <li className={selectedSet === null ? 'builder-card' : 'builder-card active'}>
      <div className="builder-card-head">
        <span className="builder-number">{index + 1}</span>
        {exercise.thumbnail_url ? (
          <img className="thumb small" src={exercise.thumbnail_url} alt="" />
        ) : (
          <span className="thumb small thumb-empty" aria-hidden="true">
            <CameraIcon />
          </span>
        )}
        <span className="builder-card-name">{exercise.name}</span>
        <button
          type="button"
          className="icon-button"
          aria-label={`Дії: ${exercise.name}`}
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen(!menuOpen)}
        >
          <MoreIcon />
        </button>
      </div>

      <div className="set-chips">
        {exercise.sets.map((set, setIndex) => (
          <button
            key={set.id}
            type="button"
            className="set-chip"
            aria-pressed={selectedSet === setIndex}
            aria-label={`${exercise.name}, підхід ${setIndex + 1}: ${setLabel(set)}`}
            onClick={() => onSelectSet(setIndex)}
          >
            <span className="set-chip-number">{setIndex + 1}</span>
            {setLabel(set)}
          </button>
        ))}
        <button
          type="button"
          className="set-chip add"
          aria-label={`Додати підхід: ${exercise.name}`}
          onClick={onAddSet}
        >
          <PlusIcon />
        </button>
        {exercise.per_side_label && <span className="tag tag-warn">{exercise.per_side_label}</span>}
      </div>
      {exercise.sets.length === 0 && (
        <p className="error small">Додай хоча б один підхід.</p>
      )}

      {noteOpen && (
        <label className="note-field">
          <span>Нотатка</span>
          <input
            value={exercise.note ?? ''}
            placeholder="що важливо в техніці"
            maxLength={500}
            onChange={(event) => onNote(event.target.value || null)}
          />
        </label>
      )}

      {menuOpen && (
        <div className="card-menu">
          <button type="button" disabled={index === 0} onClick={closeMenuAnd(() => onMove(-1))}>
            Вище
          </button>
          <button
            type="button"
            disabled={index === count - 1}
            onClick={closeMenuAnd(() => onMove(1))}
          >
            Нижче
          </button>
          <button type="button" onClick={closeMenuAnd(() => setNoteOpen(true))}>
            Нотатка
          </button>
          <button type="button" className="danger" onClick={closeMenuAnd(onRemove)}>
            Прибрати
          </button>
        </div>
      )}
    </li>
  )
}
