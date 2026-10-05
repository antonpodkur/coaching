import { AnimatePresence, m } from 'motion/react'
import { type Ref, useState } from 'react'

import { CameraIcon, MoreIcon, PlusIcon } from '../../shared/icons'
import { POP } from '../../shared/motion'
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
  /** Only for an exercise added to this workout alone. */
  onAddToLibrary?: () => void
  onRemove: () => void
  /** For the builder's list, to glide the card into place as others come and go. */
  ref?: Ref<HTMLLIElement>
}

/** A set chip pops in when added and shrinks away when removed. */
const CHIP = {
  layout: true,
  initial: { opacity: 0, scale: 0.6 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.6, transition: { duration: 0.15 } },
  whileTap: { scale: 0.92 },
  transition: POP,
} as const

/**
 * One exercise in the builder: its sets as chips, Dasha's comment for the
 * client right under them, and a ⋯ menu for moving and removing (and, for an
 * exercise added to this workout only, for adding it to the library).
 */
export function ExerciseCard({
  exercise,
  index,
  count,
  selectedSet,
  onSelectSet,
  onAddSet,
  onMove,
  onNote,
  onAddToLibrary,
  onRemove,
  ref,
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false)
  // A new comment stays open while she types, even before it has any text.
  const [writingNote, setWritingNote] = useState(false)
  const closeMenuAnd = (action: () => void) => () => {
    setMenuOpen(false)
    action()
  }

  return (
    <m.li
      ref={ref}
      className={selectedSet === null ? 'builder-card' : 'builder-card active'}
      layout="position"
      initial={{ opacity: 0, scale: 0.96 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.96 }}
    >
      <div className="builder-card-head">
        <span className="builder-number">{index + 1}</span>
        {exercise.thumbnail_url ? (
          <img className="thumb small" src={exercise.thumbnail_url} alt="" />
        ) : (
          <span className="thumb small thumb-empty" aria-hidden="true">
            <CameraIcon />
          </span>
        )}
        <span className="builder-card-name">
          {exercise.name}
          {!exercise.in_library && (
            <span className="builder-last-time">Лише в цьому тренуванні</span>
          )}
          {exercise.last_time && (
            <span className="builder-last-time">Минулого разу: {exercise.last_time}</span>
          )}
        </span>
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
        <AnimatePresence initial={false} mode="popLayout">
          {exercise.sets.map((set, setIndex) => (
            <m.button
              key={set.id}
              type="button"
              className="set-chip"
              aria-pressed={selectedSet === setIndex}
              aria-label={`${exercise.name}, підхід ${setIndex + 1}: ${setLabel(set, exercise.measure)}`}
              onClick={() => onSelectSet(setIndex)}
              {...CHIP}
            >
              <span className="set-chip-number">{setIndex + 1}</span>
              {setLabel(set, exercise.measure)}
            </m.button>
          ))}
          <m.button
            key="add"
            type="button"
            className="set-chip add"
            aria-label={`Додати підхід: ${exercise.name}`}
            onClick={onAddSet}
            {...CHIP}
          >
            <PlusIcon />
          </m.button>
          {exercise.per_side_label && (
            <m.span key="side" className="tag tag-warn" {...CHIP} whileTap={undefined}>
              {exercise.per_side_label}
            </m.span>
          )}
        </AnimatePresence>
      </div>
      {exercise.sets.length === 0 && (
        <p className="error small">Додай хоча б один підхід.</p>
      )}

      {writingNote || exercise.note !== null ? (
        <label className="note-field">
          <span>Коментар до вправи · клієнт бачить його</span>
          <textarea
            value={exercise.note ?? ''}
            placeholder="Наприклад: тримай спину рівно, не поспішай униз"
            maxLength={500}
            rows={2}
            // Focus only a comment she just asked for, not every saved one.
            autoFocus={writingNote && exercise.note === null}
            onChange={(event) => onNote(event.target.value.trim() ? event.target.value : null)}
            onBlur={() => setWritingNote(false)}
          />
        </label>
      ) : (
        <button type="button" className="note-add" onClick={() => setWritingNote(true)}>
          <PlusIcon />
          Коментар до вправи
        </button>
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
          {onAddToLibrary && (
            <button type="button" onClick={closeMenuAnd(onAddToLibrary)}>
              Додати в бібліотеку
            </button>
          )}
          <button type="button" className="danger" onClick={closeMenuAnd(onRemove)}>
            Прибрати
          </button>
        </div>
      )}
    </m.li>
  )
}
