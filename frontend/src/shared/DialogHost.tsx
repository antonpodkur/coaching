import { AnimatePresence } from 'motion/react'
import { useEffect } from 'react'
import { useLocation } from 'react-router'

import { BottomNote } from './BottomNote'
import { SheetFrame } from './Sheet'
import { type OpenAsk, type ShownNote, closeDialogs, dismissNote, useDialogs } from './dialogs'
import { AlertIcon, CloseIcon } from './icons'

/**
 * Draws the app's questions and notes (`dialogs.ts`), once, at the root.
 * Leaving the screen, by the back button too, answers no and drops the note.
 */
export function DialogHost() {
  const { ask, note } = useDialogs()
  const { pathname } = useLocation()

  useEffect(() => closeDialogs, [pathname])

  return (
    <>
      <AnimatePresence>{ask && <ConfirmSheet key={ask.title} ask={ask} />}</AnimatePresence>
      <BottomNote show={note !== null} className={note?.error ? 'flash-note failed' : 'flash-note'}>
        {note && <NoteContent note={note} />}
      </BottomNote>
    </>
  )
}

/**
 * The question in a sheet: what goes, what follows, the action in red and
 * «Скасувати» nearest the thumb. «Скасувати» has the focus, so Enter is safe.
 */
function ConfirmSheet({ ask }: { ask: OpenAsk }) {
  const no = () => ask.answer(false)

  return (
    <SheetFrame
      titleId="confirm-title"
      messageId={ask.detail ? 'confirm-detail' : undefined}
      onClose={no}
      fit
      className="confirm-sheet"
      grip={
        <div className="confirm-head">
          {ask.image && <img className="confirm-image" src={ask.image} alt="" draggable={false} />}
          <h2 id="confirm-title">{ask.title}</h2>
          {ask.detail && <p id="confirm-detail">{ask.detail}</p>}
        </div>
      }
    >
      <div className="confirm-actions">
        <button type="button" className="button block danger" onClick={() => ask.answer(true)}>
          {ask.action}
        </button>
        <button type="button" className="button block filled" onClick={no} autoFocus>
          Скасувати
        </button>
      </div>
    </SheetFrame>
  )
}

function NoteContent({ note }: { note: ShownNote }) {
  return (
    <>
      {note.error && <AlertIcon />}
      <span className="flash-note-text">{note.text}</span>
      {note.undo ? (
        <button
          type="button"
          className="button small"
          onClick={() => {
            note.undo?.()
            dismissNote(note.id)
          }}
        >
          Повернути
        </button>
      ) : (
        <button
          type="button"
          className="icon-button"
          aria-label="Закрити"
          onClick={() => dismissNote(note.id)}
        >
          <CloseIcon />
        </button>
      )}
    </>
  )
}
