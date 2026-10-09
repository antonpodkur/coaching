import { useSyncExternalStore } from 'react'

/** A question before something is deleted or removed. */
export interface Ask {
  /** The question, e.g. «Видалити це фото?». */
  title: string
  /** What follows, in a sentence, e.g. «Клієнти його більше не побачать.». */
  detail?: string
  /** The button that does it, e.g. «Видалити». */
  action: string
  /** What goes away, when it is one of several: a photo, or a video's thumbnail. */
  image?: string | null
}

/** A short note at the bottom that goes by itself. */
export interface Note {
  text: string
  /** Something failed: said in the danger color. */
  error?: boolean
  /** Puts back what was just done, under «Повернути». */
  undo?: () => void
}

export interface OpenAsk extends Ask {
  answer: (yes: boolean) => void
}

export interface ShownNote extends Note {
  id: number
}

interface Dialogs {
  ask: OpenAsk | null
  note: ShownNote | null
}

/** How long a note stays: an undo a little longer, to reach it. */
const NOTE_MS = 5000
const UNDO_MS = 6000

let dialogs: Dialogs = { ask: null, note: null }
const listeners = new Set<() => void>()
let lastNote = 0
let noteTimer: number | undefined

function change(next: Partial<Dialogs>) {
  dialogs = { ...dialogs, ...next }
  for (const listener of listeners) listener()
}

/**
 * Asks in a sheet at the bottom (`DialogHost`): resolves `true` for the
 * action, `false` for «Скасувати», a tap outside, or leaving the screen.
 */
export function confirmAction(ask: Ask): Promise<boolean> {
  // Nothing draws the sheet (never expected): the browser's own question.
  if (listeners.size === 0) {
    return Promise.resolve(window.confirm([ask.title, ask.detail].filter(Boolean).join('\n')))
  }
  // A second question replaces one left open.
  dialogs.ask?.answer(false)
  const asker = document.activeElement
  return new Promise((resolve) => {
    const open: OpenAsk = {
      ...ask,
      answer: (yes) => {
        if (dialogs.ask !== open) return resolve(yes)
        change({ ask: null })
        // Back to the button that asked, for a keyboard.
        if (asker instanceof HTMLElement && asker.isConnected) asker.focus({ preventScroll: true })
        resolve(yes)
      },
    }
    change({ ask: open })
  })
}

/** Shows a note at the bottom in place of the one before, and hides it after a while. */
export function flashNote(note: Note) {
  const shown = { ...note, id: ++lastNote }
  window.clearTimeout(noteTimer)
  noteTimer = window.setTimeout(() => dismissNote(shown.id), note.undo ? UNDO_MS : NOTE_MS)
  change({ note: shown })
}

/** Hides the note: only note `id` if given, so a newer one stays. */
export function dismissNote(id?: number) {
  if (!dialogs.note || (id !== undefined && dialogs.note.id !== id)) return
  window.clearTimeout(noteTimer)
  change({ note: null })
}

/** The open question and the shown note, for `DialogHost`. */
export function useDialogs(): Dialogs {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
    () => dialogs,
  )
}

/** Says no to the open question and hides the note, e.g. on leaving the screen. */
export function closeDialogs() {
  dialogs.ask?.answer(false)
  dismissNote()
}

/** The same question wherever someone signs out. */
export function confirmSignOut(): Promise<boolean> {
  return confirmAction({
    title: 'Вийти з акаунта?',
    detail: 'Лише на цьому пристрої. Щоб повернутися, увійди знову через Telegram.',
    action: 'Вийти',
  })
}
